use evalexpr::HashMapContext;
use murrelet_common::*;

use crate::{
    expr::{ExprWorldContextValues, IntoExprWorldContext, MixedEvalDefs},
    types::{AdditionalContextNode, LivecodeResult},
    unitcells::UnitCellContext,
};

#[derive(Debug, Clone)]
enum LivecodeWorldStateStage {
    Timeless,
    World(LiveCodeTimeInstantInfo),
    Unit(LiveCodeTimeInstantInfo),
    Lazy(LiveCodeTimeInstantInfo),
}
impl LivecodeWorldStateStage {
    fn add_step(&self, stage: LivecodeWorldStateStage) -> LivecodeWorldStateStage {
        // todo, i could start to represent the tree of steps.. but right now, just do the latest one
        stage
    }
}

#[derive(Debug, Clone)]
pub struct LivecodeWorldState {
    context: HashMapContext,
    stage: LivecodeWorldStateStage,
    assets: AssetsRef,
}
impl LivecodeWorldState {
    fn clone_ctx_and_add_world(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
        maybe_time: Option<LiveCodeTimeInstantInfo>,
        maybe_node: Option<AdditionalContextNode>,
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
            node.eval_raw(&mut ctx)?;
        }

        Ok(ctx)
    }

    pub fn new<'a>(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
        time: LiveCodeTimeInstantInfo,
        node: AdditionalContextNode,
        assets: AssetsRef,
    ) -> LivecodeResult<LivecodeWorldState> {
        let context =
            Self::clone_ctx_and_add_world(evalexpr_func_ctx, livecode_src, Some(time), Some(node))?;

        Ok(LivecodeWorldState {
            context,
            stage: LivecodeWorldStateStage::World(time),
            assets: assets.clone(),
        })
    }

    pub fn new_timeless(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
    ) -> LivecodeResult<LivecodeWorldState> {
        let context = Self::clone_ctx_and_add_world(evalexpr_func_ctx, livecode_src, None, None)?;

        Ok(LivecodeWorldState {
            context,
            stage: LivecodeWorldStateStage::Timeless,
            assets: Assets::empty_ref(),
        })
    }

    // this should use the cached one if it exists, or return an error
    pub(crate) fn ctx(&self) -> &HashMapContext {
        &self.context
    }

    pub fn time(&self) -> LiveCodeTimeInstantInfo {
        // basically always world should have time, except when computing
        // the time component.
        match self.stage {
            LivecodeWorldStateStage::Timeless => panic!("checking time in a timeless world"),
            LivecodeWorldStateStage::World(t) => t,
            LivecodeWorldStateStage::Unit(t) => t,
            LivecodeWorldStateStage::Lazy(t) => t,
        }
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
        &mut self.context
    }

    pub fn update_with_defs(&mut self, more_defs: &MixedEvalDefs) -> LivecodeResult<()> {
        more_defs.update_ctx(&mut self.ctx_mut())
    }

    pub fn clone_with_vals(
        &self,
        expr: ExprWorldContextValues,
        prefix: &str,
    ) -> LivecodeResult<LivecodeWorldState> {
        let mut lazy = self.clone_to_lazy(); // eh just need to clone

        expr.with_prefix(prefix).update_ctx(&mut lazy.ctx_mut())?;

        Ok(lazy)
    }

    pub fn clone_to_unitcell(
        &self,
        unit_cell_ctx: &UnitCellContext,
        prefix: &str,
    ) -> LivecodeResult<LivecodeWorldState> {
        let mut context = self.context.clone();
        unit_cell_ctx
            .as_expr_world_context_values()
            .with_prefix(prefix)
            .update_ctx(&mut context)?;

        let r = LivecodeWorldState {
            context,
            stage: self
                .stage
                .add_step(LivecodeWorldStateStage::Unit(self.time())),
            assets: self.assets.clone(),
        };

        Ok(r)
    }

    pub fn clone_to_lazy(&self) -> Self {
        let context = self.context.clone();
        LivecodeWorldState {
            context,
            stage: self
                .stage
                .add_step(LivecodeWorldStateStage::Lazy(self.time())),
            assets: self.assets.clone(),
        }
    }

    pub fn asset_layer(&self, key: &str, layer_idx: usize) -> Option<Vec<Polyline>> {
        self.assets.asset_layer(key, layer_idx).map(|x| x.clone())
    }

    pub fn asset_layers_in_key(&self, key: &str) -> &[String] {
        self.assets.layer_for_key(key)
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
        let seconds = self.seconds_from_config(system_timing);
        self.seconds_to_beats(seconds)
    }

    fn seconds_to_beats(&self, s: f32) -> f32 {
        let minutes = s / 60.0;
        minutes * self.bpm
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
                LivecodeValue::Float(ease(
                    time.into(),
                    (1.0 / self.timing_config.beats_per_bar).into(),
                    0.0,
                )),
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
