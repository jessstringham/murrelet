#![allow(dead_code)]
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use murrelet_common::{print_expect, IsLivecodeSrc, LivecodeValue, MurreletTime};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

// every time you call update in your program, what's the max number of times you
// check for updates from midi?
const MAX_MIDI_CHECKS_PER_UPDATE: usize = 100;

// right now this is just set as the max number of buttons and we add them all..
const MIDI_COUNT: usize = 16;

const MIDI_FIGHTER_TWISTER_NAME: &'static str = "Midi Fighter Twister";
const MIDI_FIGHTER_SPECTRA_NAME: &'static str = "Midi Fighter Spectra";
const NANO_KONTROL_NAME: &'static str = "nanoKONTROL2 SLIDER/KNOB";

// const MIDI_MNG_REGEX = Regex::new(r"^\d+$").unwrap();

impl IsLivecodeSrc for MidiMng {
    fn update(&mut self, _: &murrelet_common::LivecodeSrcUpdateInput) {
        self.values.reset();
        // run through as many updates as we can
        for _ in 0..MAX_MIDI_CHECKS_PER_UPDATE {
            let r = self.cxn.check_and_maybe_update(&mut self.values);
            if r.is_err() {
                break;
            } // leave early
        }
    }

    fn to_exec_funcs(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        let midi = &self.values.dials;
        let midi_bool = &self.values.pads;
        let midi_fire = &self.values.pads_changed;

        let mut vals = Vec::with_capacity(MIDI_COUNT * 3);
        for idx in 0..MIDI_COUNT {
            vals.push((format!("m{}", idx), LivecodeValue::Float(midi[idx] as f64)));
            vals.push((
                format!("m{}t", idx),
                LivecodeValue::Bool(midi_bool[idx] % 2 == 1),
            ));
            vals.push((format!("m{}f", idx), LivecodeValue::Bool(midi_fire[0])));
        }
        vals
    }

    fn feedback(&mut self, variables: &HashMap<String, murrelet_common::LivecodeUsage>) {
        if let Some(out) = self.out.get_mut(&MidiDevice::MidiTwister) {
            let mut twister = TwisterController { out };

            for i in 0..16u8 {
                if let Some(u) = variables.get(&format!("m{}", i)) {
                    let amount = if let Some(v) = u.value {
                        v
                    } else {
                        self.values.dials[i as usize]
                    };
                    let val = (amount * 128.0) as u8;

                    twister.set_encoder(i, val);
                    twister.set_led(i, 0x7F);
                } else {
                    twister.set_encoder(i, 0);
                    twister.set_led(i, 0);
                };
            }
        }
    }
}

struct TwisterController<'a> {
    out: &'a mut MidiOutputConnection,
}
impl<'a> TwisterController<'a> {
    fn set_led(&mut self, dial: u8, amount: u8) {
        let msg = &[0xB1, dial, amount];
        self.out.send(&msg[..]).map_err(|x| x.to_string()).ok();
    }

    fn set_encoder(&mut self, dial: u8, amount: u8) {
        let msg = &[0xB0, dial, amount];
        self.out.send(&msg[..]).map_err(|x| x.to_string()).ok();
    }
}

pub struct MidiMng {
    cxn: MidiCxn,
    pub values: MidiValues,
    out: HashMap<MidiDevice, MidiOutputConnection>,
}

impl MidiMng {
    pub fn new() -> MidiMng {
        let cxn = MidiCxn::new();
        let out = get_midi_out();
        MidiMng {
            cxn,
            values: MidiValues::new(16, 16, 16), // eh, just take the max of devices
            out,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MidiValues {
    dial_count: usize,
    dials: Vec<f32>,
    dials_changed: Vec<bool>,
    pad_count: usize,
    pads: Vec<usize>,
    pads_changed: Vec<bool>,
    last_update: u64,
    fighter: Vec<usize>,
    fighter_times: Vec<Option<MurreletTime>>,
    #[allow(dead_code)]
    fighter_count: usize,
}

impl MidiValues {
    pub fn dials(&self) -> Vec<f32> {
        self.dials.clone()
    }

    fn new(dial_count: usize, pad_count: usize, fighter_count: usize) -> Self {
        MidiValues {
            dial_count,
            dials: vec![0.5; pad_count],
            dials_changed: vec![false; pad_count],
            pad_count,
            pads: vec![0; pad_count],
            pads_changed: vec![false; pad_count],
            last_update: 0,
            fighter: vec![0; fighter_count],
            fighter_times: vec![None; fighter_count],
            fighter_count,
        }
    }

    pub fn reset(&mut self) {
        self.dials_changed = vec![false; self.dial_count];
        self.pads_changed = vec![false; self.pad_count];
    }

    pub fn pads_bool(&self, idx: usize) -> bool {
        self.pads_changed[idx]
    }

    pub fn pads_cycle(&self, idx: usize, cycle_size: usize) -> usize {
        self.pads[idx] % cycle_size
    }

    pub fn updated_since_reset(&self) -> bool {
        self.dials_changed.iter().any(|x| *x) || self.pads_changed.iter().any(|x| *x)
    }

    pub fn pads_update(&mut self, idx: usize) {
        self.pads[idx] += 1;
        self.pads_changed[idx] = true;
    }

    pub fn dials_update(&mut self, idx: usize, amount: u8) {
        self.dials[idx] = (amount as f32) / 128.0;
        self.dials_changed[idx] = true;
    }

    pub fn fighter_update(&mut self, idx: usize) {
        self.fighter[idx] += 1;
        self.fighter_times[idx] = Some(MurreletTime::now()); // could do the time we receive but ah
    }

    pub fn release_fighter_update(&mut self, idx: usize) {
        self.fighter[idx] += 1;
        self.fighter_times[idx] = Some(MurreletTime::now()); // could do the time we receive but ah
    }

    pub fn update(&mut self, msg: &MidiMessage) {
        match msg.key {
            Some(KeyPress::Pad(idx)) => {
                if msg.stamp - self.last_update > 100 * 1000 {
                    // 100 ms seems to work
                    self.pads_update(idx.into());
                }
            }
            Some(KeyPress::Dial(idx, amount)) => {
                self.dials_update(idx.into(), amount);
            }
            Some(KeyPress::ReleasePad(_)) => {
                // nothing yet
            }
            Some(KeyPress::Fight(idx)) => {
                self.fighter_update(idx.into());
            }
            Some(KeyPress::ReleaseFight(idx)) => {
                self.release_fighter_update(idx.into());
            }
            None => {}
        }
        self.last_update = msg.stamp;
    }
}

#[derive(Debug)]
pub enum KeyPress {
    ReleasePad(u8),
    Pad(u8),
    ReleaseFight(u8),
    Fight(u8),
    Dial(u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MidiDevice {
    Akai,
    MidiFighter,
    MidiTwister,
    NanoKontrol2,
    Unknown,
}
impl MidiDevice {
    fn from_name(s: Option<String>) -> Self {
        match s.as_deref() {
            Some(MIDI_FIGHTER_TWISTER_NAME) => MidiDevice::MidiTwister,
            Some(MIDI_FIGHTER_SPECTRA_NAME) => MidiDevice::MidiFighter,
            Some(NANO_KONTROL_NAME) => MidiDevice::NanoKontrol2,
            _ => MidiDevice::Unknown,
        }
    }

    fn to_str(&self) -> String {
        match self {
            MidiDevice::Akai => "akai",
            MidiDevice::MidiFighter => "midi-fighter",
            MidiDevice::MidiTwister => "midi-twister",
            MidiDevice::NanoKontrol2 => "nano-kontrol2",
            MidiDevice::Unknown => "unknown",
        }
        .to_string()
    }
}

// eh right now this only really works with one midi device at a time
// i should probably do this differently
#[derive(Debug)]
pub struct MidiMessage {
    pub device: MidiDevice,
    pub stamp: u64,
    pub key: Option<KeyPress>,
}
impl MidiMessage {
    pub fn new(device: MidiDevice, stamp: u64, message: &[u8]) -> MidiMessage {
        // ah, hm, some of these overlap...
        // also ideally this could be a config of some kind

        let key = match device {
            MidiDevice::Akai => match message {
                [_, n] => Some(KeyPress::Pad(*n)),
                [176, n @ 70..=77, value] => Some(KeyPress::Dial(n - 70, *value)),
                _ => {
                    println!("akai missed {:?}", message);
                    None
                }
            },
            MidiDevice::MidiFighter => match message {
                [128, n @ 36..=51, _value] => Some(KeyPress::ReleaseFight(n - 36)),
                [144, n @ 36..=51, _value] => Some(KeyPress::Fight(n - 36)),
                // [146, n @ 36..=51, _value] => Some(KeyPress::Pad(n - 36)),
                _ => {
                    println!("midi fighter missed {:?}", message);
                    None
                }
            },
            MidiDevice::MidiTwister => match message {
                [176, n @ 0..=15, value] => Some(KeyPress::Dial(*n, *value)),
                [177, n @ 0..=15, 127] => Some(KeyPress::Pad(*n)),
                [177, n @ 0..=15, 0] => Some(KeyPress::ReleasePad(*n)),
                _ => {
                    println!("midi twister missed {:?}", message);
                    None
                }
            },
            MidiDevice::NanoKontrol2 => {
                match message {
                    [176, n @ 0..=8, value] => Some(KeyPress::Dial(*n, *value)),
                    [176, n @ 16..=23, value] => Some(KeyPress::Dial(n - 8, *value)),
                    // S
                    [176, n @ 32..=39, _value @ 127] => Some(KeyPress::Pad(n - 32)),

                    // M
                    [176, n @ 48..=55, _value @ 127] => Some(KeyPress::Pad(n - 48 + 8)),
                    // R
                    [176, n @ 64..=71, _value @ 127] => Some(KeyPress::Pad(n - 64 + 8)),
                    _ => {
                        println!("nano missed {:?}", message);
                        None
                    }
                }
            }
            MidiDevice::Unknown => todo!(),
        };

        MidiMessage { device, stamp, key }
    }
}

pub struct MidiConn {
    device: MidiDevice,
    input: MidiInputConnection<Sender<MidiMessage>>,
}

fn connect_one_midi(
    device: MidiDevice,
    in_port: &midir::MidiInputPort,
    event_tx: Sender<MidiMessage>,
) -> Option<MidiConn> {
    let this_midi_in = MidiInput::new(&format!("midir-from-{}", device.to_str())).ok()?;

    let maybe_conn_in = this_midi_in.connect(
        in_port,
        &format!("midi-in-{}", device.to_str()),
        move |stamp, message, event_tx: &mut Sender<MidiMessage>| {
            // println!("{}: {:?} (len = {})", stamp, message, message.len());
            let m = MidiMessage::new(device, stamp, message);
            print_expect(event_tx.send(m), "error sending midi");
        },
        event_tx.clone(),
    );

    let conn_in = maybe_conn_in.ok()?;

    Some(MidiConn {
        device,
        input: conn_in,
    })
}

// borrowed from the midir example
fn connect_midi(event_tx: Sender<MidiMessage>) -> Vec<MidiConn> {
    // set up one just to get the port list
    let maybe_midi_in = MidiInput::new("midir-to-list-ports");

    if let Some(mut midi_in) = maybe_midi_in.ok() {
        midi_in.ignore(Ignore::None);

        let in_ports = midi_in.ports();

        let mut result = vec![];
        for in_port in in_ports.iter() {
            let name = midi_in.port_name(in_port).ok();
            let device = MidiDevice::from_name(name);
            if let Some(c) = connect_one_midi(device, in_port, event_tx.clone()) {
                result.push(c);
            }
        }

        result
    } else {
        println!("issue with midi!");
        vec![]
    }
}

fn get_midi_out() -> HashMap<MidiDevice, MidiOutputConnection> {
    let mut hm = HashMap::new();
    let maybe_midi_out = MidiOutput::new("midir-to-list-ports");
    if let Some(midi_out) = maybe_midi_out.ok() {
        let out_ports = midi_out.ports();

        for out_port in &out_ports {
            let name = midi_out.port_name(out_port).ok();
            let device = MidiDevice::from_name(name);

            let this_midi_out = MidiOutput::new(&format!("midir-to-{}", device.to_str())).ok();

            if let Some(mo) = this_midi_out {
                let maybe_conn_out = mo.connect(out_port, &format!("midi-out-{}", device.to_str()));

                if let Ok(connection) = maybe_conn_out {
                    hm.insert(device, connection);
                }
            }
        }
    }
    hm
}

pub struct MidiCxn {
    _midi_cxn: JoinHandle<()>, // keep it alive!
    pub rx: Receiver<MidiMessage>,
}
impl Default for MidiCxn {
    fn default() -> Self {
        Self::new()
    }
}
impl MidiCxn {
    pub fn new() -> MidiCxn {
        // set up a thread
        let (event_tx, event_rx) = mpsc::channel::<MidiMessage>();

        let handle = thread::spawn(move || {
            let _conn_in = connect_midi(event_tx);
            loop {
                thread::sleep(Duration::from_micros(100));
            }
        });

        MidiCxn {
            _midi_cxn: handle,
            rx: event_rx,
        }
    }

    pub fn check_and_maybe_update(&self, midi: &mut MidiValues) -> Result<(), mpsc::TryRecvError> {
        self.rx.try_recv().map(|x| midi.update(&x))
    }
}
