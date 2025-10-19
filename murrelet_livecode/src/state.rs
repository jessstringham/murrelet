use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use evalexpr::{Context, EvalexprResult, HashMapContext, IterateVariablesContext, Value};
use murrelet_common::*;

use crate::{
    expr::{
        lc_val_to_expr, ExprWorldContextValues, IntoExprWorldContext, MixedEvalDefs,
        MixedEvalDefsRef,
    },
    types::{AdditionalContextNode, LivecodeResult},
    unitcells::UnitCellContext,
};

#[derive(Debug, Clone)]
enum LivecodeWorldStateStage {
    Timeless,
    World(LiveCodeTimeInstantInfo),
    // Unit(LiveCodeTimeInstantInfo),
    // Lazy(LiveCodeTimeInstantInfo),
}
// impl LivecodeWorldStateStage {
//     fn add_step(&self, stage: LivecodeWorldStateStage) -> LivecodeWorldStateStage {
//         // todo, i could start to represent the tree of steps.. but right now, just do the latest one
//         stage
//     }
// }

#[derive(Clone, Debug)]
pub enum CacheFlag {
    NotCached,
    Cached,
}

#[derive(Clone, Debug)]
pub struct CachedHM {
    flag: CacheFlag,
    data: Arc<HashMapContext>, // we keep this around so we don't need to drop
}
impl CachedHM {
    fn new_rw() -> Arc<RwLock<CachedHM>> {
        let hm = HashMapContext::new();
        Arc::new(RwLock::new(CachedHM {
            data: Arc::new(hm),
            flag: CacheFlag::NotCached,
        }))
    }

    // fn update(&mut self, hm: HashMapContext) {
    //     // *self = CachedHM::Cached(Arc::new(hm));
    //     self.data = Arc::new(hm);
    //     self.flag = CacheFlag::Cached;
    // }

    fn clear(&mut self) {
        self.flag = CacheFlag::NotCached;
    }

    fn cached(&self) -> CacheResult {
        match self.flag {
            CacheFlag::NotCached => CacheResult::NotCached,
            CacheFlag::Cached => CacheResult::Cached(self.data.clone()),
        }
    }

    fn update_arc(&mut self, hm: Arc<HashMapContext>) {
        self.data = hm;
        self.flag = CacheFlag::Cached;
    }
}

enum CacheResult {
    Cached(Arc<HashMapContext>),
    NotCached,
}

#[derive(Clone, Debug)]
pub struct LivecodeWorldState {
    cached: Arc<RwLock<CachedHM>>,
    state: Arc<LivecodeWorldStateInner>,
    refs: Vec<MixedEvalDefsRef>,
}
impl LivecodeWorldState {
    pub fn new_legacy(state: LivecodeWorldStateInner) -> LivecodeResult<Self> {
        Ok(Self {
            cached: CachedHM::new_rw(),
            state: Arc::new(state),
            refs: vec![],
        })
    }

    pub fn new(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
        time: LiveCodeTimeInstantInfo,
        node: AdditionalContextNode,
        assets: AssetsRef,
    ) -> LivecodeResult<Self> {
        let state =
            LivecodeWorldStateInner::new(evalexpr_func_ctx, livecode_src, time, node, assets)?;

        Self::new_legacy(state)
    }

    pub fn new_timeless(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
    ) -> LivecodeResult<Self> {
        let state = LivecodeWorldStateInner::new_timeless(evalexpr_func_ctx, livecode_src)?;
        Self::new_legacy(state)
    }

    pub fn clone_with_vals(&self, expr: ExprWorldContextValues, prefix: &str) -> Self {
        let e = expr.with_prefix(prefix);
        let new_info = MixedEvalDefs::new_from_expr(e);

        let mut refs = self.refs.clone();
        refs.push(MixedEvalDefsRef::new(new_info));

        Self {
            cached: CachedHM::new_rw(),
            state: self.state.clone(),
            refs,
        }
    }

    pub fn clone_to_unitcell(
        &self,
        unit_cell_ctx: &UnitCellContext,
        prefix: &str,
        maybe_node: Option<&MixedEvalDefsRef>,
    ) -> LivecodeResult<LivecodeWorldState> {
        let new_info = unit_cell_ctx
            .as_expr_world_context_values()
            .with_prefix(prefix);

        let mut refs = self.refs.clone();
        refs.push(MixedEvalDefsRef::new(MixedEvalDefs::new_from_expr(
            new_info,
        )));
        if let Some(node) = maybe_node {
            refs.push(node.clone());
        }

        Ok(Self {
            cached: CachedHM::new_rw(),
            state: self.state.clone(),
            refs,
        })
    }

    pub(crate) fn new_dummy() -> Self {
        Self::new_legacy(LivecodeWorldStateInner::new_dummy()).unwrap()
    }

    pub(crate) fn ctx(&self) -> LivecodeResult<Arc<HashMapContext>> {
        if let CacheResult::Cached(c) = self.cached.read().unwrap().cached() {
            return Ok(c);
        }
        let mut cache = self.cached.write().unwrap();
        let mut ctx = self.state.ctx().clone();
        for mixed in &self.refs {
            mixed.update_ctx(&mut ctx)?;
        }
        let arc = Arc::new(ctx);
        cache.update_arc(arc.clone());

        Ok(arc)
    }

    pub fn actual_frame_u64(&self) -> u64 {
        self.state.actual_frame_u64()
    }

    pub fn vars(&self) -> HashSet<String> {
        self.state.vars()
    }

    pub fn update_with_defs(&mut self, md: MixedEvalDefsRef) {
        self.refs.push(md);
        self.cached.write().unwrap().clear();
    }

    pub fn time(&self) -> LiveCodeTimeInstantInfo {
        self.state.time()
    }

    pub fn actual_frame(&self) -> f32 {
        self.state.actual_frame()
    }

    pub fn asset_layer(&self, key: &str, layer_idx: usize) -> Option<Vec<Polyline>> {
        self.state.asset_layer(key, layer_idx)
    }

    pub fn asset_layers_in_key(&self, key: &str) -> &[String] {
        self.state.asset_layers_in_key(key)
    }

    pub(crate) fn update_with_simple_defs(
        &self,
        more_vals: ExprWorldContextValues,
    ) -> WorldWithLocalVariables {
        let locals = more_vals.to_vals();

        WorldWithLocalVariables {
            base: self.ctx().unwrap(),
            locals,
            builtins_disabled: false,
        }
    }

    pub fn to_local(&self) -> WorldWithLocalVariables {
        WorldWithLocalVariables {
            base: self.ctx().unwrap(),
            locals: vec![],
            builtins_disabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldWithLocalVariables {
    base: Arc<HashMapContext>, // your cached global ctx (world.ctx()?.as_ref())
    locals: Vec<(String, Value)>, // small slice like &[("loc_pct", 0.42.into())]
    builtins_disabled: bool,
}
impl WorldWithLocalVariables {
    pub fn update_with_simple_defs(&mut self, more_vals: &ExprWorldContextValues) {
        let mut locals = more_vals.to_vals();
        locals.extend(self.locals.iter().cloned());
        self.locals = locals;

        // WorldWithLocalVariables {
        //     base: self.base.clone(),
        //     locals,
        //     builtins_disabled: true,
        // }
    }

    pub(crate) fn variable_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.locals.iter().map(|(k, _)| k.clone()).collect();
        names.extend(self.base.iter_variable_names());
        names
    }
}

impl Context for WorldWithLocalVariables {
    fn get_value(&self, identifier: &str) -> Option<&Value> {
        // locals win
        if let Some((_, v)) = self.locals.iter().find(|(k, v)| k == identifier) {
            return Some(v);
        }
        // otherwise fallback to global
        self.base.get_value(identifier)
    }

    fn call_function(&self, identifier: &str, argument: &Value) -> EvalexprResult<Value> {
        self.base.call_function(identifier, argument)
    }

    fn are_builtin_functions_disabled(&self) -> bool {
        self.builtins_disabled
    }

    fn set_builtin_functions_disabled(&mut self, disabled: bool) -> EvalexprResult<()> {
        self.builtins_disabled = disabled;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LivecodeWorldStateInner {
    context: HashMapContext,
    stage: LivecodeWorldStateStage,
    assets: AssetsRef,
}
impl LivecodeWorldStateInner {
    pub fn vars(&self) -> HashSet<String> {
        self.context.iter_variable_names().collect()
    }

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

    pub fn new(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
        time: LiveCodeTimeInstantInfo,
        node: AdditionalContextNode,
        assets: AssetsRef,
    ) -> LivecodeResult<Self> {
        let context =
            Self::clone_ctx_and_add_world(evalexpr_func_ctx, livecode_src, Some(time), Some(node))?;

        Ok(Self {
            context,
            stage: LivecodeWorldStateStage::World(time),
            assets: assets.clone(),
        })
    }

    pub fn new_timeless(
        evalexpr_func_ctx: &HashMapContext,
        livecode_src: &LivecodeSrc,
    ) -> LivecodeResult<Self> {
        let context = Self::clone_ctx_and_add_world(evalexpr_func_ctx, livecode_src, None, None)?;

        Ok(Self {
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
            // LivecodeWorldStateStage::Unit(t) => t,
            // LivecodeWorldStateStage::Lazy(t) => t,
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
        more_defs.update_ctx(self.ctx_mut())
    }

    // pub fn clone_with_vals(
    //     &self,
    //     expr: ExprWorldContextValues,
    //     prefix: &str,
    // ) -> LivecodeResult<LivecodeWorldStateRef> {
    //     let mut lazy = self.clone_to_lazy(); // eh just need to clone
    //     expr.with_prefix(prefix).update_ctx(lazy.ctx_mut())?;
    //     Ok(lazy)
    // }

    // pub fn clone_to_unitcell(
    //     &self,
    //     unit_cell_ctx: &UnitCellContext,
    //     prefix: &str,
    // ) -> LivecodeResult<LivecodeWorldState> {
    //     let mut context = self.context.clone();
    //     unit_cell_ctx
    //         .as_expr_world_context_values()
    //         .with_prefix(prefix)
    //         .update_ctx(&mut context)?;

    //     let r = LivecodeWorldState {
    //         context,
    //         stage: self
    //             .stage
    //             .add_step(LivecodeWorldStateStage::Unit(self.time())),
    //         assets: self.assets.clone(),
    //     };

    //     Ok(r)
    // }

    // pub fn clone_to_lazy(&self) -> Self {
    //     let context = self.context.clone();
    //     LivecodeWorldState {
    //         context,
    //         stage: self
    //             .stage
    //             .add_step(LivecodeWorldStateStage::Lazy(self.time())),
    //         assets: self.assets.clone(),
    //     }
    // }

    pub fn asset_layer(&self, key: &str, layer_idx: usize) -> Option<Vec<Polyline>> {
        self.assets.asset_layer(key, layer_idx).cloned()
    }

    pub fn asset_layers_in_key(&self, key: &str) -> &[String] {
        self.assets.layer_for_key(key)
    }

    pub fn new_dummy() -> Self {
        let empty_ctx = HashMapContext::new();
        Self::new(
            &empty_ctx,
            &LivecodeSrc::new(vec![]),
            LiveCodeTimeInstantInfo::new_dummy(), // time
            AdditionalContextNode::new_dummy(),   // node
            Arc::new(Assets::empty()),            // assets
        )
        .unwrap()
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

    fn new_dummy() -> LiveCodeTimeInstantInfo {
        LiveCodeTimeInstantInfo {
            timing_config: LivecodeTimingConfig {
                bpm: 120.0,
                fps: 60.0,
                realtime: false,
                beats_per_bar: 4.0,
            },
            system_timing: LiveCodeTiming::default(),
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
                LivecodeValue::Float(ease(time.into(), (1.0 / 4.0).into(), 0.0)),
            ),
            (
                "stease".to_owned(),
                LivecodeValue::Float(ease(time.into(), 0.0125, 0.0)),
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
