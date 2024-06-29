//#![allow(dead_code)]
use midir::{Ignore, MidiInput, MidiInputConnection};
use murrelet_common::{print_expect, IsLivecodeSrc, LivecodeValue};
use std::error::Error;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

// every time you call update in your program, what's the max number of times you
// check for updates from midi?
const MAX_MIDI_CHECKS_PER_UPDATE: usize = 100;

// right now this is just set as the max number of buttons and we add them all..
const MIDI_COUNT: usize = 16;

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

    fn to_just_midi(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        let mut vals = Vec::with_capacity(MIDI_COUNT * 3);
        for idx in 0..MIDI_COUNT {
            vals.push((format!("m{}", idx), LivecodeValue::Float(0.0)));
            vals.push((format!("m{}t", idx), LivecodeValue::Bool(false)));
            vals.push((format!("m{}f", idx), LivecodeValue::Bool(false)));
        }
        vals
    }
}

pub struct MidiMng {
    cxn: MidiCxn,
    pub values: MidiValues,
}

impl MidiMng {
    pub fn new() -> MidiMng {
        let cxn = MidiCxn::new();
        MidiMng {
            cxn,
            values: MidiValues::new(16, 16), // eh, just take the max of devices
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
}

impl MidiValues {
    pub fn dials(&self) -> Vec<f32> {
        self.dials.clone()
    }

    fn new(dial_count: usize, pad_count: usize) -> Self {
        MidiValues {
            dial_count,
            dials: MidiValues::_get_n_0_5(dial_count),
            dials_changed: MidiValues::_get_n_false(dial_count),
            pad_count,
            pads: MidiValues::_get_n_usize0(pad_count),
            pads_changed: MidiValues::_get_n_false(pad_count),
            last_update: 0,
        }
    }

    pub fn new_midi_fighter() -> Self {
        MidiValues::new(0, 16)
    }

    pub fn new_akai() -> Self {
        MidiValues::new(8, 8)
    }

    pub fn reset(&mut self) {
        self.dials_changed = MidiValues::_get_n_false(self.dial_count);
        self.pads_changed = MidiValues::_get_n_false(self.pad_count);
    }

    fn _get_n_0_5(count: usize) -> Vec<f32> {
        (0..count).map(|_| 0.5).collect::<Vec<f32>>()
    }

    fn _get_n_usize0(count: usize) -> Vec<usize> {
        (0..count).map(|_| 0).collect::<Vec<usize>>()
    }

    fn _get_n_false(count: usize) -> Vec<bool> {
        (0..count).map(|_| false).collect::<Vec<bool>>()
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
            None => {}
        }
        self.last_update = msg.stamp;
    }
}

#[derive(Debug)]
pub enum KeyPress {
    Pad(u8),
    Dial(u8, u8),
}

#[derive(Debug)]
pub enum MidiDevice {
    Akai,
    MidiFighter,
    MidiTwister,
    NanoKontrol2,
}

// eh right now this only really works with one midi device at a time
// i should probably do this differently
#[derive(Debug)]
pub struct MidiMessage {
    pub stamp: u64,
    pub source: Option<MidiDevice>,
    pub key: Option<KeyPress>,
}
impl MidiMessage {
    pub fn new(stamp: u64, message: &[u8]) -> MidiMessage {
        // ah, hm, some of these overlap...
        // also ideally this could be a config of some kind
        let (source, key) = match message.len() {
            2 => match (message[0], message[1]) {
                (_, n) => (Some(MidiDevice::Akai), Some(KeyPress::Pad(n))),
            },
            3 => match (message[0], message[1], message[2]) {
                (176, n @ 70..=77, value) => {
                    (Some(MidiDevice::Akai), Some(KeyPress::Dial(n - 70, value)))
                }
                (146, n @ 36..=51, _value) => {
                    (Some(MidiDevice::MidiFighter), Some(KeyPress::Pad(n - 36)))
                }
                // also 0..7 are nano slides
                (176, n @ 0..=15, value) => (
                    Some(MidiDevice::MidiTwister),
                    Some(KeyPress::Dial(n, value)),
                ),
                (176, n @ 16..=23, value) => (
                    Some(MidiDevice::NanoKontrol2),
                    Some(KeyPress::Dial(n - 8, value)),
                ),
                // S
                (176, n @ 32..=39, _value @ 127) => {
                    (Some(MidiDevice::NanoKontrol2), Some(KeyPress::Pad(n - 32)))
                }
                // M
                (176, n @ 48..=55, _value @ 127) => (
                    Some(MidiDevice::NanoKontrol2),
                    Some(KeyPress::Pad(n - 48 + 8)),
                ),
                // R
                (176, n @ 64..=71, _value @ 127) => (
                    Some(MidiDevice::NanoKontrol2),
                    Some(KeyPress::Pad(n - 64 + 8)),
                ),
                (177, n @ 0..=15, 127) => (Some(MidiDevice::MidiTwister), Some(KeyPress::Pad(n))),
                _ => (None, None),
            },
            _ => (None, None),
        };
        MidiMessage { stamp, source, key }
    }
}

// borrowed from the midir example
fn connect_midi(event_tx: Sender<MidiMessage>) -> Result<MidiInputConnection<()>, Box<dyn Error>> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);
    let in_ports = midi_in.ports();
    let in_port_o = match in_ports.len() {
        0 => None,
        1 => Some(&in_ports[0]),
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            Some(
                in_ports
                    .get(input.trim().parse::<usize>()?)
                    .ok_or("invalid input port selected")?,
            )
        }
    };
    let in_port = in_port_o.unwrap();

    let conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |stamp, message, _| {
            // println!("{}: {:?} (len = {})", stamp, message, message.len());
            let m = MidiMessage::new(stamp, message);
            print_expect(event_tx.send(m), "error sending midi");
        },
        (),
    )?;

    Ok(conn_in)
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
            let _conn_in = connect_midi(event_tx).unwrap();
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
