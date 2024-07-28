#![allow(dead_code)]
use evalexpr::build_operator_tree;
use evalexpr::Node;
use glam::vec2;
use glam::vec3;
use glam::Vec2;
use glam::Vec3;
use murrelet_common::clamp;
use murrelet_common::ease;
use murrelet_common::map_range;

use murrelet_common::IsLivecodeSrc;
use murrelet_common::LivecodeSrc;
use murrelet_common::LivecodeValue;
use murrelet_common::MurreletColor;
use murrelet_common::MurreletTime;
use serde::Deserialize;

use crate::unitcells::LazyNodeF32;
use crate::unitcells::LazyNodeF32Def;
use crate::unitcells::{
    EvaluableUnitCell, UnitCellControlExprBool, UnitCellControlExprF32, UnitCellEvalContext,
};

// for default values
pub fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

pub trait LivecodeFromWorld<T> {
    fn o(&self, w: &LiveCodeWorldState) -> T;

    fn just_midi(&self, w: &TimelessLiveCodeWorldState) -> T;
}

impl LivecodeFromWorld<Vec2> for [ControlF32; 2] {
    fn o(&self, w: &LiveCodeWorldState) -> Vec2 {
        vec2(self[0].o(w), self[1].o(w))
    }

    fn just_midi(&self, w: &TimelessLiveCodeWorldState) -> Vec2 {
        vec2(self[0].just_midi(w), self[1].just_midi(w))
    }
}

impl LivecodeFromWorld<Vec3> for [ControlF32; 3] {
    fn o(&self, w: &LiveCodeWorldState) -> Vec3 {
        vec3(self[0].o(w), self[1].o(w), self[2].o(w))
    }

    fn just_midi(&self, w: &TimelessLiveCodeWorldState) -> Vec3 {
        vec3(
            self[0].just_midi(w),
            self[1].just_midi(w),
            self[2].just_midi(w),
        )
    }
}

impl LivecodeFromWorld<MurreletColor> for [ControlF32; 4] {
    fn o(&self, w: &LiveCodeWorldState) -> MurreletColor {
        // by default, clamp saturation and value
        MurreletColor::hsva(
            self[0].o(w),
            clamp(self[1].o(w), 0.0, 1.0),
            clamp(self[2].o(w), 0.0, 1.0),
            self[3].o(w),
        )
    }

    fn just_midi(&self, w: &TimelessLiveCodeWorldState) -> MurreletColor {
        MurreletColor::hsva(
            self[0].just_midi(w),
            clamp(self[1].just_midi(w), 0.0, 1.0),
            clamp(self[2].just_midi(w), 0.0, 1.0),
            self[3].just_midi(w),
        )
    }
}

pub trait LivecodeToControl<ControlT> {
    fn to_control(&self) -> ControlT;
}

impl LivecodeToControl<ControlF32> for f32 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self)
    }
}

impl LivecodeToControl<ControlF32> for i32 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlF32> for u32 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlF32> for u8 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlBool> for bool {
    fn to_control(&self) -> ControlBool {
        ControlBool::Raw(*self)
    }
}

impl LivecodeToControl<[ControlF32; 2]> for Vec2 {
    fn to_control(&self) -> [ControlF32; 2] {
        [self.x.to_control(), self.y.to_control()]
    }
}

impl LivecodeToControl<[ControlF32; 3]> for Vec3 {
    fn to_control(&self) -> [ControlF32; 3] {
        [
            self.x.to_control(),
            self.y.to_control(),
            self.z.to_control(),
        ]
    }
}

impl LivecodeToControl<[ControlF32; 4]> for MurreletColor {
    fn to_control(&self) -> [ControlF32; 4] {
        let [r, g, b, a] = self.into_rgba_components();
        [
            r.to_control(),
            g.to_control(),
            b.to_control(),
            a.to_control(),
        ]
    }
}

impl LivecodeToControl<ControlF32> for usize {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlF32> for u64 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<LazyNodeF32Def> for LazyNodeF32 {
    fn to_control(&self) -> LazyNodeF32Def {
        LazyNodeF32Def::new(self.n().cloned().unwrap())
    }
}

// i don't know if this is a good place to put this...
pub fn _auto_default_f32_0() -> ControlF32 {
    ControlF32::Raw(0.0)
}
pub fn _auto_default_f32_1() -> ControlF32 {
    ControlF32::Raw(1.0)
}

pub fn _auto_default_vec2_0() -> [ControlF32; 2] {
    [ControlF32::Raw(0.0), ControlF32::Raw(0.0)]
}
pub fn _auto_default_vec2_1() -> [ControlF32; 2] {
    [ControlF32::Raw(1.0), ControlF32::Raw(1.0)]
}

pub fn _auto_default_bool_false() -> ControlBool {
    ControlBool::Raw(false)
}
pub fn _auto_default_bool_true() -> ControlBool {
    ControlBool::Raw(true)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ControlF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    Expr(Node),
}

impl ControlF32 {
    // for backwards compatibility
    #[allow(non_snake_case)]
    pub fn Raw(v: f32) -> ControlF32 {
        Self::Float(v)
    }

    pub fn force_from_str(s: &str) -> ControlF32 {
        match build_operator_tree(s) {
            Ok(e) => Self::Expr(e),
            Err(err) => {
                println!("{:?}", err);
                ControlF32::Raw(1.0)
            }
        }
    }

    pub fn to_unitcell_control(&self) -> UnitCellControlExprF32 {
        match self {
            ControlF32::Int(x) => UnitCellControlExprF32::Int(*x),
            ControlF32::Bool(x) => UnitCellControlExprF32::Bool(*x),
            ControlF32::Float(x) => UnitCellControlExprF32::Float(*x),
            ControlF32::Expr(x) => UnitCellControlExprF32::Expr(x.clone()),
        }
    }

    pub fn o(&self, w: &LiveCodeWorldState) -> f32 {
        let world_context = UnitCellEvalContext::from_world(w);

        match self.to_unitcell_control().eval(&world_context) {
            Ok(x) => x,
            Err(err) => {
                println!("{}", err);
                1.0
            }
        }
    }

    pub fn just_midi(&self, m: &TimelessLiveCodeWorldState) -> f32 {
        let world_context = UnitCellEvalContext::from_timeless(m);
        match self.to_unitcell_control().eval(&world_context) {
            Ok(x) => x,
            Err(err) => {
                println!("{}", err);
                1.0
            }
        }
    }

    // pub fn vec3(c: &[ControlF32; 3], w: &LiveCodeWorldState) -> Vec3 {
    //     vec3(c[0].o(w), c[1].o(w), c[2].o(w))
    // }

    // pub fn array3(c: &[ControlF32; 3], w: &LiveCodeWorldState) -> [f32; 3] {
    //     [c[0].o(w), c[1].o(w), c[2].o(w)]
    // }

    // pub fn array4(c: &[ControlF32; 4], w: &LiveCodeWorldState) -> [f32; 4] {
    //     [c[0].o(w), c[1].o(w), c[2].o(w), c[3].o(w)]
    // }

    // pub fn hsva(c: &[ControlF32; 4], w: &LiveCodeWorldState) -> MurreletColor {
    //     let c = ControlF32::array4(c, w);
    //     // gonna clamp saturation and value
    //     hsva(c[0], clamp(c[1], 0.0, 1.0), clamp(c[2], 0.0, 1.0), c[3]).into_lin_srgba()
    // }

    // pub fn hsva_unclamped(c: &[ControlF32; 4], w: &LiveCodeWorldState) -> MurreletColor {
    //     let c = ControlF32::array4(c, w);
    //     // gonna clamp just value
    //     hsva(c[0], c[1], c[2], c[3]).into_lin_srgba()
    // }

    // pub fn hsva_more_info(c: &[ControlF32; 4], w: &LiveCodeWorldState) -> [f32; 4] {
    //     let (r, g, b, a) = ControlF32::hsva(c, w).into_components();
    //     [r, g, b, a]
    // }

    // pub fn vec2_midi(c: &[ControlF32; 2], w: &TimelessLiveCodeWorldState) -> Vec2 {
    //     vec2(c[0].just_midi(w), c[1].just_midi(w))
    // }

    // pub fn vec3_midi(c: &[ControlF32; 3], w: &TimelessLiveCodeWorldState) -> Vec3 {
    //     vec3(c[0].just_midi(w), c[1].just_midi(w), c[2].just_midi(w))
    // }

    // pub fn array3_midi(c: &[ControlF32; 3], w: &TimelessLiveCodeWorldState) -> [f32; 3] {
    //     [c[0].just_midi(w), c[1].just_midi(w), c[2].just_midi(w)]
    // }

    pub fn array4_midi(c: &[ControlF32; 4], w: &TimelessLiveCodeWorldState) -> [f32; 4] {
        [
            c[0].just_midi(w),
            c[1].just_midi(w),
            c[2].just_midi(w),
            c[3].just_midi(w),
        ]
    }

    // pub fn hsva_midi(c: &[ControlF32; 4], w: &TimelessLiveCodeWorldState) -> MurreletColor {
    //     let c = ControlF32::array4_midi(c, w);
    //     // gonna clamp saturation and value
    //     hsva(c[0], clamp(c[1], 0.0, 1.0), clamp(c[2], 0.0, 1.0), c[3]).into_lin_srgba()
    // }

    pub fn hsva_unclamped_midi(
        c: &[ControlF32; 4],
        w: &TimelessLiveCodeWorldState,
    ) -> MurreletColor {
        let c = ControlF32::array4_midi(c, w);
        MurreletColor::hsva(c[0], c[1], c[2], c[3])
    }

    // pub fn hsva_more_info_midi(c: &[ControlF32; 4], w: &TimelessLiveCodeWorldState) -> [f32; 4] {
    //     let (r, g, b, a) = ControlF32::hsva_midi(c, w).into_components();
    //     [r, g, b, a]
    // }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ControlBool {
    Raw(bool),
    Int(i32),
    Float(f32),
    Expr(Node),
}
impl ControlBool {
    pub fn to_unitcell_control(&self) -> UnitCellControlExprBool {
        match self {
            ControlBool::Raw(x) => UnitCellControlExprBool::Bool(*x),
            ControlBool::Int(x) => UnitCellControlExprBool::Int(*x),
            ControlBool::Float(x) => UnitCellControlExprBool::Float(*x),
            ControlBool::Expr(x) => UnitCellControlExprBool::Expr(x.clone()),
        }
    }

    pub fn force_from_str(s: &str) -> ControlBool {
        match build_operator_tree(s) {
            Ok(e) => Self::Expr(e),
            Err(err) => {
                println!("{:?}", err);
                ControlBool::Raw(false)
            }
        }
    }

    pub fn o(&self, w: &LiveCodeWorldState) -> bool {
        let world_context = UnitCellEvalContext::from_world(w);

        match self.to_unitcell_control().eval(&world_context) {
            Ok(x) => x,
            Err(err) => {
                println!("{}", err);
                false
            }
        }
    }

    pub fn just_midi(&self, m: &TimelessLiveCodeWorldState) -> bool {
        let world_context = UnitCellEvalContext::from_timeless(m);
        match self.to_unitcell_control().eval(&world_context) {
            Ok(x) => x,
            Err(err) => {
                println!("{}", err);
                false
            }
        }
    }

    pub fn default(&self) -> bool {
        match self {
            ControlBool::Raw(x) => *x,
            ControlBool::Int(x) => *x > 0,
            ControlBool::Float(x) => *x > 0.0,
            ControlBool::Expr(_) => false, // eh
        }
    }
}

pub struct LiveCodeWorldState<'a> {
    pub idx: f32,
    pub livecode_src: &'a LivecodeSrc,
    pub time: LiveCodeTimeInstantInfo,
    pub ctx: Node,
}
impl<'a> LiveCodeWorldState<'a> {
    pub fn new(
        livecode_src: &'a LivecodeSrc,
        time: LiveCodeTimeInstantInfo,
        ctx: Node,
    ) -> LiveCodeWorldState<'a> {
        LiveCodeWorldState {
            idx: 0.0, // is this used?
            livecode_src,
            time,
            ctx,
        }
    }

    pub fn to_world_vals(&self) -> Vec<(String, LivecodeValue)> {
        let mut w = self.livecode_src.to_world_vals();
        w.extend(self.time.to_exec_funcs());
        w
    }

    pub fn actual_frame(&self) -> f32 {
        self.time.actual_frame()
    }

    pub fn actual_frame_u64(&self) -> u64 {
        self.time.actual_frame_u64()
    }

    pub fn should_debug(&self) -> bool {
        self.time.seconds_since_updated_realtime() < 0.1
    }
}

// todo, maybe rethink this, maybe can remove it from LivecodeTimingConfig..
pub struct TimelessLiveCodeWorldState<'a> {
    livecode_src: &'a LivecodeSrc,
}
impl<'a> TimelessLiveCodeWorldState<'a> {
    pub fn new(livecode_src: &'a LivecodeSrc) -> TimelessLiveCodeWorldState<'a> {
        TimelessLiveCodeWorldState { livecode_src }
    }

    pub fn to_timeless_vals(&self) -> Vec<(String, LivecodeValue)> {
        self.livecode_src.to_timeless_vals()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LivecodeTimingConfig {
    pub bpm: f32,
    pub fps: f32,
    pub realtime: bool,
    pub beats_per_bar: f32,
}
impl LivecodeTimingConfig {
    fn seconds_from_config(&self, system_timing: LiveCodeTiming) -> f32 {
        if self.realtime {
            self.current_time_seconds_realtime(system_timing)
        } else {
            self.current_time_seconds_frame(system_timing)
        }
    }

    fn beat_from_config(&self, system_timing: LiveCodeTiming) -> f32 {
        //time_from_config(self.bpm, self.realtime, actual_frame, self.fps, system_timing.start)
        let seconds = self.seconds_from_config(system_timing);
        self.seconds_to_beats(seconds)
    }

    fn seconds_to_beats(&self, s: f32) -> f32 {
        let minutes = s / 60.0;
        minutes * self.bpm
    }

    fn beats_to_bar(&self, beats: f32) -> f32 {
        beats / self.beats_per_bar
    }

    fn seconds_to_bar(&self, s: f32) -> f32 {
        self.beats_to_bar(self.seconds_to_beats(s))
    }

    fn current_time_seconds_realtime(&self, system_timing: LiveCodeTiming) -> f32 {
        (MurreletTime::now() - system_timing.start).as_secs_f32()
    }

    fn current_time_seconds_frame(&self, system_timing: LiveCodeTiming) -> f32 {
        system_timing.frame as f32 / self.fps
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LiveCodeTimeInstantInfo {
    timing_config: LivecodeTimingConfig,
    system_timing: LiveCodeTiming,
}
impl LiveCodeTimeInstantInfo {
    pub fn new(
        timing_config: LivecodeTimingConfig,
        system_timing: LiveCodeTiming,
    ) -> LiveCodeTimeInstantInfo {
        LiveCodeTimeInstantInfo {
            timing_config,
            system_timing,
        }
    }

    pub fn debug(&self) -> String {
        format!(
            "realtime: {}\nseconds: {:.01}\nbeat: {:.01}\nbar: {:.01} ({})\nframe: {}\nlast updated {:?}",
            self.timing_config.realtime,
            self.seconds(),
            self.beat(),
            self.bar(),
            self.is_on_bar(),
            self.actual_frame(),
            self.system_timing.last_config_update
        )
    }

    pub fn actual_frame_u64(&self) -> u64 {
        self.system_timing.frame
    }

    pub fn actual_frame(&self) -> f32 {
        self.system_timing.frame as f32
    }

    // magical
    pub fn beat(&self) -> f32 {
        self.timing_config.beat_from_config(self.system_timing)
    }

    pub fn bar(&self) -> f32 {
        self.beat() / self.timing_config.beats_per_bar
    }

    pub fn seconds(&self) -> f32 {
        self.timing_config.seconds_from_config(self.system_timing)
    }

    pub fn is_on_bar(&self) -> bool {
        // okay so, we want to know the prev beat.
        let prev_time = if self.timing_config.realtime {
            let render_time = self.system_timing.last_render_time;
            render_time.as_secs_f32()
        } else {
            let prev_frame = self.system_timing.frame - 1;
            prev_frame as f32 / self.timing_config.fps
        };

        // check if this beat rounds differently than the curr one
        let prev_beat = self.timing_config.seconds_to_beats(prev_time);

        let curr_beat_bar = self.bar().floor();
        let prev_beat_bar = (prev_beat / self.timing_config.beats_per_bar).floor();

        curr_beat_bar as i32 > prev_beat_bar as i32
    }

    pub fn seconds_since_updated_realtime(&self) -> f32 {
        // check when last updated
        let last_update_time = self.system_timing.last_config_update;
        // check how much time has passed since it was last updated
        let time = MurreletTime::now() - last_update_time;
        time.as_secs_f32()
    }

    fn seconds_since_updated_frame(&self) -> f32 {
        // check when last updated
        let last_update_time = self.system_timing.last_config_update_frame;
        // check how much time has passed since it was last updated
        let frames_since_updated = self.system_timing.frame - last_update_time;

        (frames_since_updated as f32) / self.timing_config.fps
    }

    pub fn seconds_between_render_times(&self) -> f32 {
        if self.timing_config.realtime {
            (self.system_timing.last_render_time - self.system_timing.prev_render_time)
                .as_secs_f32()
        } else {
            1.0 / self.timing_config.fps
        }
    }

    pub fn seconds_since_reload(&self) -> f32 {
        if self.timing_config.realtime {
            self.seconds_since_updated_realtime()
        } else {
            self.seconds_since_updated_frame()
        }
    }

    fn bars_since_reload(&self) -> f32 {
        let sec_since_updated = self.seconds_since_reload();

        // and convert to bars
        self.timing_config.seconds_to_bar(sec_since_updated)
    }

    fn beat_scaled_offset(&self, mult: f32, offset: f32) -> f32 {
        (self.beat() * mult) + offset
    }

    fn beat_scaled_fract(&self, mult: f32) -> f32 {
        self.beat_scaled(mult).fract()
    }

    fn beat_scaled_idx(&self, mult: f32, count: usize) -> usize {
        self.beat_scaled(mult).floor() as usize % count
    }

    fn beat_scaled(&self, mult: f32) -> f32 {
        self.beat_scaled_offset(mult, 0.0)
    }

    fn beat_scaled_ramp_min_max(&self, mult: f32, min: f32, max: f32) -> f32 {
        map_range(self.beat_scaled_fract(mult), 0.0, 1.0, min, max)
    }

    fn beat_scaled_min_max(&self, mult: f32, min: f32, max: f32) -> f32 {
        map_range(self.beat_scaled(mult), 0.0, 1.0, min, max)
    }
}

impl IsLivecodeSrc for LiveCodeTimeInstantInfo {
    fn update(&mut self, input: &murrelet_common::LivecodeSrcUpdateInput) {
        self.system_timing.frame = input.app().elapsed_frames();
    }

    fn to_exec_funcs(&self) -> Vec<(String, LivecodeValue)> {
        let time = self.beat();
        let frame = self.actual_frame_u64();

        vec![
            ("t".to_owned(), LivecodeValue::Float(time as f64)),
            (
                "tease".to_owned(),
                LivecodeValue::Float(ease(time.into(), 0.2, 0.0)),
            ),
            (
                "stease".to_owned(),
                LivecodeValue::Float(ease(time.into(), 0.01, 0.0)),
            ),
            ("ti".to_owned(), LivecodeValue::Int(time as i64)),
            ("f".to_owned(), LivecodeValue::Float(frame as f64)),
            ("fi".to_owned(), LivecodeValue::Int(frame as i64)),
        ]
    }

    fn to_just_midi(&self) -> Vec<(String, LivecodeValue)> {
        let time = self.beat();
        let frame = self.actual_frame_u64();

        vec![
            ("t".to_owned(), LivecodeValue::Float(time as f64)),
            (
                "tease".to_owned(),
                LivecodeValue::Float(ease(time.into(), 0.2, 0.0)),
            ),
            (
                "stease".to_owned(),
                LivecodeValue::Float(ease(time.into(), 0.01, 0.0)),
            ),
            ("ti".to_owned(), LivecodeValue::Int(time as i64)),
            ("f".to_owned(), LivecodeValue::Float(frame as f64)),
            ("fi".to_owned(), LivecodeValue::Int(frame as i64)),
        ]
    }
}

pub struct LiveCodeConfigInfo {
    pub config_next_check: MurreletTime,
    updated: bool,
}

// LoadableDrawConfig::load_if_needed(self.config_next_check)

impl Default for LiveCodeConfigInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveCodeConfigInfo {
    pub fn new() -> LiveCodeConfigInfo {
        LiveCodeConfigInfo {
            config_next_check: MurreletTime::in_one_sec(),
            updated: true,
        }
    }

    pub fn should_check(&self) -> bool {
        MurreletTime::now() > self.config_next_check
    }

    pub fn reset(&mut self) {
        self.updated = false;
    }

    pub fn update(&mut self, updated: bool, config_next_check: MurreletTime) {
        self.updated = updated;
        self.config_next_check = config_next_check;
    }

    pub fn updated(&self) -> bool {
        self.updated
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LiveCodeTiming {
    start: MurreletTime,
    frame: u64,
    start_frame: u64,
    true_start: MurreletTime,         // not updated when reset
    last_config_update: MurreletTime, // i don't know, config could also have it
    last_config_update_frame: u64,
    last_render_time: MurreletTime, // used for realtime bar update, also to measure time between frames for simulations
    prev_render_time: MurreletTime, // used for simulations
}

impl Default for LiveCodeTiming {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveCodeTiming {
    pub fn new() -> LiveCodeTiming {
        LiveCodeTiming {
            start: MurreletTime::now(),
            frame: 0,
            start_frame: 0,
            true_start: MurreletTime::now(),
            last_config_update: MurreletTime::now(),
            last_config_update_frame: 0,
            last_render_time: MurreletTime::now(),
            prev_render_time: MurreletTime::now(),
        }
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn true_start(&self) -> MurreletTime {
        self.true_start
    }

    pub fn config_updated(&mut self) {
        self.last_config_update = MurreletTime::now();
        self.last_config_update_frame = self.frame; // copy over curr frame
    }

    pub fn reset_time(&mut self) {
        self.start = MurreletTime::now();
        self.start_frame = self.frame;
    }

    pub fn set_last_render_time(&mut self) {
        self.prev_render_time = self.last_render_time;
        self.last_render_time = MurreletTime::now();
    }

    pub fn set_frame(&mut self, frame: u64) {
        self.frame = frame - self.start_frame;
    }
}
