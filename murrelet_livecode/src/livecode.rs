#![allow(dead_code)]
use evalexpr::build_operator_tree;
use evalexpr::EvalexprError;
use evalexpr::HashMapContext;
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

use crate::expr::ExprWorldContextValues;
use crate::unitcells::LazyNodeF32;
use crate::unitcells::LazyNodeF32Def;
use crate::unitcells::{
    EvaluableUnitCell, UnitCellControlExprBool, UnitCellControlExprF32, UnitCellWorldState,
};

// for default values
pub fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

#[derive(Debug)]
pub enum LivecodeError {
    Raw(String), // my custom errors
    EvalExpr(String, EvalexprError),
}
impl LivecodeError {}
impl std::fmt::Display for LivecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LivecodeError::Raw(msg) => write!(f, "{}", msg),
            LivecodeError::EvalExpr(msg, err) => write!(f, "{}: {}", msg, err),
        }
    }
}

impl std::error::Error for LivecodeError {}

pub type LivecodeResult<T> = Result<T, LivecodeError>;

// 'world is approximately one frame
pub trait LivecodeFromWorld<T> {
    fn o(&self, w: &LiveCodeWorldState) -> LivecodeResult<T>;
}

impl LivecodeFromWorld<Vec2> for [ControlF32; 2] {
    fn o(&self, w: &LiveCodeWorldState) -> LivecodeResult<Vec2> {
        Ok(vec2(self[0].o(w)?, self[1].o(w)?))
    }
}

impl LivecodeFromWorld<Vec3> for [ControlF32; 3] {
    fn o(&self, w: &LiveCodeWorldState) -> LivecodeResult<Vec3> {
        Ok(vec3(self[0].o(w)?, self[1].o(w)?, self[2].o(w)?))
    }
}

impl LivecodeFromWorld<MurreletColor> for [ControlF32; 4] {
    fn o(&self, w: &LiveCodeWorldState) -> LivecodeResult<MurreletColor> {
        // by default, clamp saturation and value
        Ok(MurreletColor::hsva(
            self[0].o(w)?,
            clamp(self[1].o(w)?, 0.0, 1.0),
            clamp(self[2].o(w)?, 0.0, 1.0),
            self[3].o(w)?,
        ))
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

    pub fn o(&self, w: &LiveCodeWorldState) -> LivecodeResult<f32> {
        let world_context = UnitCellWorldState::from_world(w);
        self.to_unitcell_control().eval(&world_context)
    }
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

    pub fn o(&self, w: &LiveCodeWorldState) -> LivecodeResult<bool> {
        let world_context = UnitCellWorldState::from_world(w);
        self.to_unitcell_control().eval(&world_context)
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

#[derive(Debug, Clone)]
pub struct LiveCodeWorldState {
    // Usually time is available, except the one moment where we're loading the config needed to generate
    // the timing config, which is needed to generate the time. That _should_ all be internal,
    time: Option<LiveCodeTimeInstantInfo>,
    cached_context: HashMapContext,
}
impl LiveCodeWorldState {
    pub fn clone_ctx_and_add_world(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
        maybe_time: Option<LiveCodeTimeInstantInfo>,
        maybe_node: Option<Node>,
    ) -> LivecodeResult<HashMapContext> {
        let mut ctx = evalexpr_func_ctx.clone();

        let mut w = livecode_src.to_world_vals();
        if let Some(time) = maybe_time {
            w.extend(time.to_exec_funcs());
        }
        // add the world to the ctx
        let vals = ExprWorldContextValues::new(w);
        vals.update_ctx(&mut ctx)?;

        // add the node to the context
        if let Some(node) = maybe_node {
            node.eval_empty_with_context_mut(&mut ctx)
                .map_err(|err| LivecodeError::EvalExpr("node eval failed".to_owned(), err))?;
        }

        Ok(ctx)
    }

    pub fn new<'a>(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
        time: LiveCodeTimeInstantInfo,
        node: Node,
    ) -> LivecodeResult<LiveCodeWorldState> {
        let cached_context =
            Self::clone_ctx_and_add_world(evalexpr_func_ctx, livecode_src, Some(time), Some(node))?;

        Ok(LiveCodeWorldState {
            time: Some(time),
            cached_context,
        })
    }

    pub fn new_timeless(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
    ) -> LivecodeResult<LiveCodeWorldState> {
        let cached_context =
            Self::clone_ctx_and_add_world(evalexpr_func_ctx, livecode_src, None, None)?;

        Ok(LiveCodeWorldState {
            time: None,
            cached_context,
        })
    }

    // this should use the cached one if it exists, or return an error
    pub(crate) fn ctx(&self) -> &HashMapContext {
        &self.cached_context
    }

    pub fn time(&self) -> LiveCodeTimeInstantInfo {
        // basically always world should have time, except when computing
        // the time component.
        self.time.expect("tried calling time on timeless world")
    }

    pub fn actual_frame(&self) -> f32 {
        self.time().actual_frame()
    }

    pub fn actual_frame_u64(&self) -> u64 {
        self.time().actual_frame_u64()
    }

    pub fn should_debug(&self) -> bool {
        self.time().seconds_since_updated_realtime() < 0.1
    }

    pub fn is_on_bar(&self) -> bool {
        self.time().is_on_bar()
    }

    pub(crate) fn ctx_mut(&mut self) -> &mut HashMapContext {
        &mut self.cached_context
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
