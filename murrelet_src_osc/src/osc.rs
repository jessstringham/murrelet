#![allow(dead_code)]
use murrelet_common::{
    print_expect, IsLivecodeSrc, LivecodeSrcUpdateInput, LivecodeUsage, LivecodeValue,
};
use rosc::{OscPacket, OscType};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::net::{SocketAddrV4, ToSocketAddrs};
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// send something to the address sent to the manager, flattened and prefixed with this, and it'll turn into a param in the livecode world!
const OSC_PREFIX: &str = "/livecode/";

impl IsLivecodeSrc for OscMng {
    fn update(&mut self, _: &LivecodeSrcUpdateInput) {
        // drain all the messages available
        let mut i = 0;
        let count = 300;
        for _ in 0..count {
            let r = self.cxn.check_and_maybe_update(&mut self.values);
            if r.is_err() {
                break;
            } // leave early
            i += 1
        }
        if i >= count - 1 {
            println!("that's a lot of osc messages to go through!");
        }
    }

    fn to_exec_funcs(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        self.values.to_livecode_vals()
    }

    fn feedback(
        &mut self,
        _variables: &HashMap<String, LivecodeUsage>,
        outgoing_msgs: &[(String, String, LivecodeValue)],
    ) {
        for (_addr, name, vals) in outgoing_msgs.iter() {
            match vals {
                LivecodeValue::Float(v) => {
                    let args: Vec<OscType> = vec![OscType::Float(*v as f32)];
                    let osc_path = format!("{}{}", OSC_PREFIX, name);

                    let msg = rosc::OscMessage {
                        addr: osc_path.to_string(),
                        args,
                    };
                    let packet_data = rosc::encoder::encode(&OscPacket::Message(msg));
                    if let Ok(pd) = packet_data {
                        if let Some(a) = &self.cxn.target_addr {
                            print_expect(
                                self.cxn.send_socket.send_to(&pd, a),
                                "failed to send osc",
                            );
                        }
                    }
                }
                LivecodeValue::Bool(_) => {}
                LivecodeValue::Int(_) => {}
            }
        }
    }
}

pub struct OscMng {
    cxn: OscCxn,
    pub values: OscValues,
}

#[derive(Debug)]
pub struct OscValues {
    last_values: HashMap<String, LivecodeValue>,
    smooth_values: HashMap<String, LivecodeValue>,
    // pub msg: Option<OSCMessage>,
}

impl OscValues {
    fn to_livecode_vals(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        let last_values: Vec<(String, murrelet_common::LivecodeValue)> =
            self.last_values.clone().into_iter().collect();
        let smooth_values: Vec<(String, murrelet_common::LivecodeValue)> = self
            .smooth_values
            .clone()
            .into_iter()
            .map(|(key, val)| (format!("{}_smooth", key), val))
            .collect();

        [last_values, smooth_values].concat()
    }
}

impl OscMng {
    pub fn new_from_str(ip_address: &str, smoothed: bool, target_addr: Option<String>) -> Self {
        let addr = match SocketAddrV4::from_str(ip_address) {
            Ok(addr) => addr,
            Err(_) => panic!(
                "address couldn't be parsed with SocketAddrV4 {}",
                ip_address
            ),
        };

        let cxn = OscCxn::new(&addr, smoothed, target_addr);
        Self {
            cxn,
            values: OscValues {
                last_values: HashMap::new(),
                smooth_values: HashMap::new(),
            },
        }
    }
}

pub struct OscCxn {
    smoothed: bool,
    _osc_cxn: JoinHandle<()>, // keep it alive!
    pub osc_rx: Receiver<OSCMessage>,
    send_socket: UdpSocket,
    target_addr: Option<String>,
}

impl OscCxn {
    pub fn check_and_maybe_update(&self, r: &mut OscValues) -> Result<(), mpsc::TryRecvError> {
        self.osc_rx.try_recv().map(|x| {
            // println!("osc message {:?}", x);

            for (name, new_val) in x.to_livecode_vals().into_iter() {
                if let Some(old_val) = r.smooth_values.get(&name) {
                    let actual_new_val = match (old_val, new_val) {
                        (LivecodeValue::Float(old), LivecodeValue::Float(new)) => {
                            if self.smoothed {
                                LivecodeValue::Float(*old * 0.9 + new * 0.1)
                            } else {
                                LivecodeValue::Float(new)
                            }
                        }
                        _ => new_val,
                    };

                    r.smooth_values.insert(name.clone(), actual_new_val);
                } else {
                    println!("first time seeing name {} with value {:?}", name, new_val);
                    r.smooth_values.insert(name.clone(), new_val);
                }

                // println!("{:?} {:?}", name, new_val);
                r.last_values.insert(name.clone(), new_val); // todo, probably good to get timestamp
            }
        })
    }

    pub fn new<A: ToSocketAddrs>(addr: &A, smoothed: bool, target_addr: Option<String>) -> Self {
        let (event_tx, event_rx) = mpsc::channel::<OSCMessage>();

        let sock = UdpSocket::bind(addr).unwrap();
        let send_socket = sock
            .try_clone()
            .expect("Failed to clone socket for sending"); // Clone the socket

        println!("setting up osc");
        println!("sock {:?}", sock);

        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; rosc::decoder::MTU];

            loop {
                match sock.recv_from(&mut buf) {
                    Ok((size, _)) => {
                        let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                        if let Some(msg) = handle_packet(&packet) {
                            print_expect(event_tx.send(msg), "error sending osc msg");
                        }
                    }
                    Err(e) => {
                        println!("Error receiving from socket: {}", e);
                        break;
                    }
                }
                thread::sleep(Duration::from_micros(100));
            }
        });

        OscCxn {
            smoothed,
            _osc_cxn: handle,
            osc_rx: event_rx,
            send_socket,
            target_addr,
        }
    }
}

#[derive(Debug)]
pub struct OSCMessage {
    v: Vec<LivecodeOSC>,
}

impl OSCMessage {
    fn new(v: Vec<LivecodeOSC>) -> Self {
        Self { v }
    }

    fn to_livecode_vals(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        self.v
            .iter()
            .map(|osc| (format!("oo_{}", osc.name), osc.v))
            .collect::<Vec<_>>()
    }
}

#[derive(Debug)]
pub struct LivecodeOSC {
    name: String,
    v: murrelet_common::LivecodeValue,
}

impl LivecodeOSC {
    pub fn new_f32(name: String, v: f32) -> Self {
        Self {
            name,
            v: LivecodeValue::float(v),
        }
    }
}

fn handle_packet(packet: &OscPacket) -> Option<OSCMessage> {
    // println!("packet {:?}", packet);
    match packet {
        OscPacket::Message(msg) => {
            if let Some(osc_name) = msg.addr.as_str().strip_prefix(OSC_PREFIX) {
                let mut values = vec![];
                match msg.args[..] {
                    [OscType::Float(value)] => {
                        values.push(LivecodeOSC::new_f32(osc_name.to_owned(), value));
                    }
                    _ => {
                        println!("OSC data values funny: {:?}", msg.args);
                    }
                }

                Some(OSCMessage::new(values))
            } else {
                println!("unexpected name, not with {}", OSC_PREFIX);

                println!("OSC address: {}", msg.addr);
                println!("OSC arguments: {:?}", msg.args);

                None
            }
        }
        OscPacket::Bundle(bundle) => {
            let mut values = vec![];

            for c in &bundle.content {
                if let OscPacket::Message(msg) = c {
                    // first check
                    if let Some(osc_name) = msg.addr.as_str().strip_prefix(OSC_PREFIX) {
                        match msg.args[..] {
                            [OscType::Float(value)] => {
                                values.push(LivecodeOSC::new_f32(osc_name.to_owned(), value));
                            }
                            _ => {
                                println!("OSC address content funny: {:?}", msg.args);
                            }
                        }
                    } else {
                        println!(
                            "OSC label dropped, not prefixed with {}: {}",
                            OSC_PREFIX, msg.addr
                        );
                    }
                }
            }

            Some(OSCMessage::new(values))
        }
    }
}
