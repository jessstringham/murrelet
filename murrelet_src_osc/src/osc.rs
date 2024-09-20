#![allow(dead_code)]
use murrelet_common::{print_expect, IsLivecodeSrc, LivecodeSrcUpdateInput, LivecodeValue};
use rosc::{OscPacket, OscType};
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
        for _ in 0..10 {
            let r = self.cxn.check_and_maybe_update(&mut self.values);
            if r.is_err() {
                break;
            } // leave early
        }
    }

    fn to_exec_funcs(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        self.values.to_livecode_vals()
    }
}

pub struct OscMng {
    cxn: OscCxn,
    pub values: OscValues,
}

#[derive(Debug)]
pub struct OscValues {
    pub msg: Option<OSCMessage>,
}

impl OscValues {
    fn to_livecode_vals(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        self.msg
            .as_ref()
            .map(|x| x.to_livecode_vals())
            .unwrap_or(vec![])
    }
}

impl OscMng {
    pub fn new_from_str(ip_address: &str) -> Self {
        let addr = match SocketAddrV4::from_str(ip_address) {
            Ok(addr) => addr,
            Err(_) => panic!(
                "address couldn't be parsed with SocketAddrV4 {}",
                ip_address
            ),
        };

        let cxn = OscCxn::new(&addr);
        Self {
            cxn,
            values: OscValues { msg: None },
        }
    }
}

pub struct OscCxn {
    _osc_cxn: JoinHandle<()>, // keep it alive!
    pub osc_rx: Receiver<OSCMessage>,
}

impl OscCxn {
    pub fn check_and_maybe_update(&self, r: &mut OscValues) -> Result<(), mpsc::TryRecvError> {
        self.osc_rx.try_recv().map(|x| {
            r.msg = Some(x);
        })
    }

    pub fn new<A: ToSocketAddrs>(addr: &A) -> Self {
        let (event_tx, event_rx) = mpsc::channel::<OSCMessage>();

        let sock = UdpSocket::bind(addr).unwrap();

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
            _osc_cxn: handle,
            osc_rx: event_rx,
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
    match packet {
        OscPacket::Message(msg) => {
            println!("not used to OscMessage! Can you send as a bundle?");
            println!("OSC address: {}", msg.addr);
            println!("OSC arguments: {:?}", msg.args);
            None
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
                                println!("OSC address content funny: {}", msg.addr);
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
