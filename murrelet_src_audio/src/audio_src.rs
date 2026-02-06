#![allow(dead_code)]
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use glam::{Vec2, vec2};
use murrelet_common::{IsLivecodeSrc, LivecodeValue, clamp, map_range};
use rustfft::{Fft, FftPlanner, num_complex::Complex, num_traits::Zero};
use std::{
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
};

struct AudioCxn {
    _stream_cxn: JoinHandle<()>,
    receiver: Receiver<AudioMessage>,
}
impl AudioCxn {
    fn new() -> Result<AudioCxn> {
        let (event_tx, event_rx) = mpsc::channel::<AudioMessage>();

        let audio_host = cpal::default_host();
        let device = audio_host
            .default_input_device()
            .expect("Failed to get default input device");
        println!("Input device: {}", device.name()?);

        // Get the default input configuration
        let config = device
            .default_input_config()
            .expect("Failed to get default input config");
        println!("Default input config: {:?}", config);

        println!("channels: {:?}", config.channels());
        println!("sample_rate: {:?}", config.sample_rate());

        let mut capture_model = CaptureModel::new(
            1024 * 2,
            // default_input_config.sample_rate().0,
            event_tx,
        );

        let handle = std::thread::spawn(move || {
            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[_], _| {
                            capture_model.process(data);
                        },
                        move |err| {
                            eprintln!("Error occurred on the input stream: {}", err);
                        },
                    )
                    .unwrap(),
                _ => panic!("i'm not sure how to handle other sample formats yet"),
            };

            // Start the stream

            stream.play().expect("failed to play");

            loop {}
        });

        Ok(AudioCxn {
            _stream_cxn: handle,
            receiver: event_rx,
        })
    }

    pub fn check_and_maybe_update(
        &self,
        audio: &mut AudioValues,
    ) -> Result<(), mpsc::TryRecvError> {
        self.receiver.try_recv().map(|x| audio.update(&x))
    }
}

// this box is meant to help us convert the frequency buckets into a
// number between 0 and 1
// or note if it's probably not that interesting.
#[derive(Debug, Clone)]
pub struct FFTStats {
    // low_noise: f32, // below this value, basically treat as 0, might say like lower 10%
    last_val: f32,
    max: f32,            // the max value seen, actual value
    max_decay_rate: f32, // the max value will decrease by this %
    smooth: f32,
    // these are in pcts
    max_bnd_pct: f32, // we will clamp max above this to 1
    min_bnd_pct: f32, // we will clamp min below this to 0
    // used to count variance
    count: usize,
    count_below_min: usize,
    count_above_max: usize,
}
impl FFTStats {
    fn new() -> Self {
        Self {
            last_val: 0.0,
            max: 0.0,
            max_decay_rate: 0.0001,
            // 1.0 is don't smooth
            // 0.0 would break
            // think it work good pretty low
            smooth: 0.01, // how much to smooth the pct with recent mean
            // this is the range we'll look for things in
            max_bnd_pct: 0.5,
            min_bnd_pct: 0.01,
            count: 0,
            count_below_min: 0,
            count_above_max: 0,
        }
    }

    fn update(&mut self, new_val: f32) -> f32 {
        self.count += 1;
        // update max, either setting it or decaying it
        if new_val > self.max {
            self.max = new_val;
        } else {
            self.max *= 1.0 - self.max_decay_rate;
        }

        // now check the current pct
        let raw_pct = new_val / self.max;

        self.last_val = raw_pct;
        self.last_val
    }

    pub fn stats(&self) -> String {
        format!(
            "max: {:0.5}, {:.0}%<min, {:.0}%>max, mean: {:.0}%",
            self.max,
            100.0 * self.count_below_min as f32 / self.count as f32,
            100.0 * self.count_above_max as f32 / self.count as f32,
            100.0 * self.last_val
        )
    }
}

#[derive(Debug, Clone)]
pub struct AudioValues {
    last_amplitude: Option<f32>,
    max_amplitude: FFTStats,
    pub fft: Option<Vec<f32>>,
    pub raw_fft: Option<FFTProcessed>,
    pub fft_stats: Vec<FFTStats>,
}
impl AudioValues {
    fn new() -> AudioValues {
        AudioValues {
            last_amplitude: None,
            max_amplitude: FFTStats::new(),
            fft: None,
            raw_fft: None,
            fft_stats: vec![FFTStats::new(); 7],
        }
    }

    fn reset(&mut self) {
        println!("resetting amp max");
        self.max_amplitude = FFTStats::new();
        self.fft_stats = vec![FFTStats::new(); 7];
    }

    fn update(&mut self, msg: &AudioMessage) {
        self.last_amplitude = msg.last_amplitude;

        if let Some(ac) = self.last_amplitude {
            self.max_amplitude.update(ac);
        }

        if let Some(new_fft) = &msg.fft {
            let mut stats = vec![0.0; 7];
            for (i, stat) in new_fft.as_array().iter().enumerate() {
                stats[i] = self.fft_stats[i].update(*stat);
            }
            self.fft = Some(stats);
        }

        self.raw_fft = msg.fft.clone();
    }

    pub fn fft(&self) -> [f32; 7] {
        if let Some(fft) = &self.fft {
            [fft[0], fft[1], fft[2], fft[3], fft[4], fft[5], fft[6]]
        } else {
            [0.0; 7]
        }
    }

    pub fn print_fft_info(&self) {
        for (i, s) in self.fft_stats.iter().enumerate() {
            println!("{}: {}", i, s.stats());
        }
    }

    pub fn amp_pct(&self) -> f32 {
        self.max_amplitude.last_val
    }
}

struct AudioMessage {
    last_amplitude: Option<f32>,
    fft: Option<FFTProcessed>,
}

// It's called FFTProcessed, but keeping around the whole graph
#[derive(Debug, Clone)]
pub struct FFTProcessed {
    pub audio: Vec<f32>,
    pub freq: Vec<f32>,
    pub length: usize,
    // pub vlow: f32, // below 60hz
    // pub low: f32, // 60hz - 250hz
    // pub lowmid: f32, // 250hz - 500hz
    // pub uppermid: f32, // 500hz - 2000hz
    // pub highmid: f32, // 2000hz - 4000hz
    // pub brilliance: f32, // 4000hz - 6000hz
    // pub superhigh: f32, // 6000hz - 20000hz
}
impl FFTProcessed {
    pub fn new(audio: &[f32], freq: &[Complex<f32>]) -> Self {
        Self {
            audio: audio.to_vec(),
            freq: freq.iter().map(|x| x.norm()).collect(),
            length: audio.len(),
        }
    }

    // also drops negative frequences
    pub fn scaled_freq(&self) -> Vec<Vec2> {
        let f_res: f32 = 44100f32 / self.length as f32;

        self.freq
            .iter()
            .take(self.length / 2 - 1)
            .enumerate()
            .map(|(k, v)| {
                let freq = k as f32 * f_res;
                vec2(freq, *v)
            })
            .collect()
    }

    pub fn zero() -> FFTProcessed {
        FFTProcessed {
            audio: vec![0.0; 1024],

            freq: vec![0.0; 1024],

            length: 1024,
            // vlow: 0.0,
            // low: 0.0,
            // lowmid: 0.0,
            // uppermid: 0.0,
            // highmid: 0.0,
            // brilliance: 0.0,
            // superhigh: 0.0,
        }
    }

    // fn new(audio: &Vec<Complex<f32>>, fft: &Vec<Complex<f32>>, sample_window_size: f32) -> FFTProcessed {

    //     // FFTProcessed {
    //     //     vlow: vals[0],
    //     //     low: vals[1],
    //     //     lowmid: vals[2],
    //     //     uppermid: vals[3],
    //     //     highmid: vals[4],
    //     //     brilliance: vals[5],
    //     //     superhigh: vals[6],
    //     // }
    // }

    pub fn as_array(&self) -> [f32; 7] {
        // [self.vlow, self.low, self.lowmid, self.uppermid, self.highmid, self.brilliance, self.superhigh]
        let mut vals = [0.0; 7];

        for x in self.scaled_freq().iter() {
            let freq = x.x;
            let v = x.y;

            if freq <= 60.0 {
                vals[0] += v;
            } else if freq <= 250.0 {
                vals[1] += v;
            } else if freq <= 500.0 {
                vals[2] += v;
            } else if freq <= 2000.0 {
                vals[3] += v;
            } else if freq <= 4000.0 {
                vals[4] += v;
            } else if freq <= 6000.0 {
                vals[5] += v;
            } else if freq <= 20000.0 {
                vals[6] += v;
            }
        }

        // vals[0] /= (60.0 - 30.0) / f_res;
        // vals[1] /= (250.0 - 60.0) / f_res;
        // vals[2] /= (500.0 - 250.0) / f_res;
        // vals[3] /= (2000.0 - 500.0) / f_res;
        // vals[4] /= (4000.0 - 2000.0) / f_res;
        // vals[5] /= (6000.0 - 4000.0) / f_res;
        // vals[6] /= (20000.0 - 6000.0) / f_res;

        vals
    }
}

struct CaptureModel {
    sample_window_size: usize,
    sender: Sender<AudioMessage>,
    curr_frames: usize, // how many samples in this window
    curr_buffer: usize, // whether to write to buffer 0 or 1 for fft
    curr_sum: f32,
    last_amplitude: Option<f32>,
    fft: Arc<dyn Fft<f32>>,
    buffer_fft: Vec<Complex<f32>>,
    buffer_audio: Vec<f32>,
    max_amplitude_per_buffer: Vec<f32>,
    max_buffer_for_amplitude: usize,
    min_max_val: f32,
}

impl CaptureModel {
    fn new(sample_window_size: usize, sender: Sender<AudioMessage>) -> CaptureModel {
        // assuming sample_window_size is a power of 2
        let mut planner = FftPlanner::new();

        let fft_size = 2 * sample_window_size;
        let fft = planner.plan_fft_forward(fft_size);

        let buffer_fft: Vec<Complex<f32>> = vec![Complex::zero(); fft_size]; // used in place for fft

        let buffer_audio: Vec<f32> = vec![0.0; fft_size]; // used for audio

        CaptureModel {
            sample_window_size,
            sender,
            curr_frames: 0,
            curr_buffer: 0,
            curr_sum: 0.0,
            last_amplitude: None,
            fft,
            buffer_fft,
            buffer_audio,
            max_amplitude_per_buffer: vec![0.0],
            max_buffer_for_amplitude: 16,
            min_max_val: 0.01, // based on the quietest noise in this room
        }
    }

    fn process_fft(&mut self) -> FFTProcessed {
        // fft stuff

        let max_amp = self.max_amp_so_far();

        let mut audio = vec![0.0; self.sample_window_size * 2];
        // hanning?
        for i in 0..self.sample_window_size * 2 {
            let s = self.buffer_audio[i] / max_amp; // normalize
            audio[i] = s;
            let alpha = 2.0 * std::f32::consts::PI * self.curr_frames as f32
                / (self.sample_window_size as f32 - 1.0);
            let window_value = 0.5 * (1.0 - alpha.cos());
            let new_s = s * window_value;

            self.buffer_fft[i] = Complex::new(new_s, 0.0);
        }

        self.fft.process(&mut self.buffer_fft);

        FFTProcessed::new(&audio, &self.buffer_fft)
    }

    fn max_amp_so_far(&self) -> f32 {
        let buffer_max = self
            .max_amplitude_per_buffer
            .iter()
            .fold(0.0_f32, |acc, &x| acc.max(x));
        (0.1f32).max(buffer_max)
    }

    fn process(&mut self, buffer: &[f32]) {
        // my microphone has 64 frames, 2 (inputs?)

        // for frame in buffer {
        // average over channels? meh, we do it anyway..
        // let frame_val = frame.into_iter().fold(0.0, |acc, x| acc + x) / frame.len() as f32;

        for s in buffer {
            self.curr_sum += s.abs();

            let new_s = *s;

            self.buffer_audio[self.curr_frames + self.sample_window_size] = new_s;
            self.max_amplitude_per_buffer[0] = self.max_amplitude_per_buffer[0].max(new_s.abs());

            self.curr_frames += 1;

            // depends on these being divisible
            if self.curr_frames >= self.sample_window_size {
                let avg_amp = self.curr_sum / self.curr_frames as f32;
                self.last_amplitude = Some(avg_amp);

                let fft = self.process_fft();

                self.sender
                    .send(AudioMessage {
                        last_amplitude: self.last_amplitude,
                        fft: Some(fft),
                    })
                    .ok();
                // reset
                self.curr_frames = 0;
                self.curr_sum = 0.0;

                self.buffer_audio.rotate_left(self.sample_window_size);

                // println!("len {:?}", self.max_amplitude_per_buffer.len());
                if self.max_amplitude_per_buffer.len() < self.max_buffer_for_amplitude {
                    self.max_amplitude_per_buffer.push(0.0);
                }
                self.max_amplitude_per_buffer.rotate_right(1);
                self.max_amplitude_per_buffer[0] = 0.0;
            }
        }
    }
}

pub struct AudioMng {
    cxn: Option<AudioCxn>,
    pub values: AudioValues,
}
impl Default for AudioMng {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioMng {
    pub fn new() -> AudioMng {
        let cxn = AudioCxn::new().ok();
        AudioMng {
            cxn,
            values: AudioValues::new(), // just take the max of all the devices?
        }
    }

    pub fn exists(&self) -> bool {
        self.cxn.is_some()
    }

    pub fn reset(&mut self) {
        self.values.reset()
    }
}

impl IsLivecodeSrc for AudioMng {
    fn update(&mut self, input: &murrelet_common::LivecodeSrcUpdateInput) {
        if input.should_reset() {
            self.reset();
        }

        if let Some(cxn) = self.cxn.as_ref() {
            for _ in 0..100 {
                // this is slow, just get the biggest one you can
                let r = cxn.check_and_maybe_update(&mut self.values);
                if r.is_err() {
                    break;
                } // leave early
            }
        }
    }

    fn to_exec_funcs(&self) -> Vec<(String, murrelet_common::LivecodeValue)> {
        let [fft0, fft1, fft2, fft3, fft4, fft5, fft6] = self.values.fft();

        let audio = self.values.amp_pct();
        let audio_clamp_raw = clamp(audio, 0.01, 0.3);
        let audio_clamp = map_range(audio_clamp_raw, 0.01, 0.3, 0.0, 1.0);

        vec![
            ("a".to_owned(), LivecodeValue::Float(audio as f64)),
            ("ac".to_owned(), LivecodeValue::Float(audio_clamp as f64)),
            ("fft0".to_owned(), LivecodeValue::Float(fft0 as f64)),
            ("fft1".to_owned(), LivecodeValue::Float(fft1 as f64)),
            ("fft2".to_owned(), LivecodeValue::Float(fft2 as f64)),
            ("fft3".to_owned(), LivecodeValue::Float(fft3 as f64)),
            ("fft4".to_owned(), LivecodeValue::Float(fft4 as f64)),
            ("fft5".to_owned(), LivecodeValue::Float(fft5 as f64)),
            ("fft6".to_owned(), LivecodeValue::Float(fft6 as f64)),
        ]
    }
}
