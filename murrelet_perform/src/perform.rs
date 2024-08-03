#![allow(dead_code)]
use glam::{vec3, Mat4, Vec2};
use murrelet_common::{LivecodeSrc, LivecodeSrcUpdateInput, MurreletAppInput};
use murrelet_common::{MurreletColor, TransformVec2};
use murrelet_livecode::boop::{BoopConfInner, BoopODEConf};
use std::{env, fs};

use murrelet_common::run_id;
use std::path::{Path, PathBuf};

use evalexpr::Node;
use murrelet_livecode::boop::{BoopConf, BoopFromWorld};
use murrelet_livecode::livecode::LivecodeFromWorld;
use murrelet_livecode::livecode::*;
use murrelet_livecode_derive::Livecode;

use crate::reload::*;

pub trait CommonTrait: std::fmt::Debug + Clone {}

// requirements for the control conf
pub trait LiveCodeCommon<T>: LivecodeFromWorld<T> + LiveCoderLoader + CommonTrait {}

// requirements for the conf
pub trait ConfCommon<T: BoopFromWorld<Self>>: CommonTrait {
    fn config_app_loc(&self) -> &AppConfig;
}

// requirements for the boop
pub trait BoopConfCommon<T>: BoopFromWorld<T> + CommonTrait {}

#[derive(Clone, Debug)]
pub struct SvgDrawConfig {
    size: f32,
    capture_path: Option<PathBuf>, // if it's missing, we don't save (e.g. web browser)
    frame: u64,
    target_size: f32, // in mm
    margin_size: f32,
}
impl SvgDrawConfig {
    pub fn new(
        size: f32,
        capture_path: Option<PathBuf>,
        target_size: f32,
        frame: u64,
    ) -> SvgDrawConfig {
        SvgDrawConfig {
            size,
            capture_path,
            target_size,
            margin_size: 10.0,
            frame,
        }
    }

    pub fn full_target_width(&self) -> f32 {
        self.target_size + 2.0 * self.margin_size
    }

    pub fn target_size(&self) -> f32 {
        self.target_size
    }

    pub fn size(&self) -> f32 {
        self.size
    }
    pub fn capture_path(&self) -> Option<PathBuf> {
        self.capture_path.clone()
    }

    pub fn transform_for_size(&self) -> Mat4 {
        // okay so we take the width, since that's what looked okay on the screen
        let size = self.size();
        let full_target_width = self.full_target_width() * 1.0;

        let translation_to_final = vec3(full_target_width, full_target_width, 0.0);
        let s = self.target_size / size;
        let scale = vec3(s, s, 1.0);

        // aiming for 100mm by 100mm, going from 0 to 10
        // operations go right to left!
        Mat4::from_translation(translation_to_final) * Mat4::from_scale(scale)
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }
}

impl TransformVec2 for SvgDrawConfig {
    fn transform_vec2(&self, v: Vec2) -> Vec2 {
        self.transform_for_size().transform_vec2(v)
    }
}

// helpful defaults you might import into yours
pub fn _default_false() -> bool {
    false
}

// app config
fn _default_seed() -> ControlF32 {
    ControlF32::Raw(42.0)
}
fn _default_width() -> ControlF32 {
    ControlF32::Raw(400.0)
}
fn _default_midi() -> ControlBool {
    ControlBool::Raw(false)
}
fn _default_audio() -> ControlBool {
    ControlBool::Raw(false)
}
fn _default_bpm() -> ControlF32 {
    ControlF32::Raw(90.0)
}
fn _default_fps() -> ControlF32 {
    ControlF32::Raw(30.0)
}
fn _default_beats_per_bar() -> ControlF32 {
    ControlF32::Raw(4.0)
}
fn _default_realtime() -> ControlBool {
    ControlBool::Raw(true)
}
fn _default_bg_alpha() -> ControlF32 {
    ControlF32::force_from_str("slog(m15, -5.0, 0.0)")
} // defaults to midi
fn _default_capture_frame() -> ControlBool {
    ControlBool::force_from_str("kSf")
} // usually want to leave this as midi
fn _default_clear_bg() -> ControlBool {
    ControlBool::force_from_str("m12") // todo, make this relax if missing
} // usually want to leave this as midi
fn _default_bg_color() -> [ControlF32; 4] {
    [
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(1.0),
    ]
}
fn _default_svg_size() -> ControlF32 {
    ControlF32::Raw(100.0)
}
fn _default_svg_save() -> ControlBool {
    ControlBool::Raw(false)
}

// this stuff adjusts how time works, so needs to be split off pretty early
#[allow(dead_code)]
#[derive(Debug, Clone, Livecode)]
pub struct AppConfigTiming {
    #[livecode(serde_default = "_default_bpm")]
    pub bpm: f32,
    #[livecode(serde_default = "_default_beats_per_bar")]
    pub beats_per_bar: f32,
    #[livecode(serde_default = "_default_fps")]
    pub fps: f32,
    #[livecode(serde_default = "_default_realtime")]
    pub realtime: bool,
}
impl AppConfigTiming {
    fn to_livecode(&self) -> LivecodeTimingConfig {
        LivecodeTimingConfig {
            bpm: self.bpm,
            beats_per_bar: self.beats_per_bar,
            fps: self.fps,
            realtime: self.realtime,
        }
    }
}

fn _default_dyn_f() -> ControlF32 {
    ControlF32::Raw(1.0)
}

fn _default_dyn_z() -> ControlF32 {
    ControlF32::Raw(1.0)
}

fn _default_dyn_r() -> ControlF32 {
    ControlF32::Raw(1.0)
}

fn _default_dyn_reset() -> ControlBool {
    ControlBool::Raw(true)
}

// this stuff adjusts how things update
#[derive(Debug, Clone, Livecode)]
pub struct AppConfigBoopODEConf {
    pub f: f32, // freq
    pub z: f32, // something
    pub r: f32, // reaction
}
impl AppConfigBoopODEConf {
    fn to_livecode(&self) -> BoopODEConf {
        BoopODEConf::new(self.f, self.z, self.r)
    }
}

#[derive(Debug, Clone, Livecode)]
pub enum AppConfigBoopConfInner {
    ODE(AppConfigBoopODEConf),
    Noop,
}
impl AppConfigBoopConfInner {
    fn to_livecode(&self) -> BoopConfInner {
        match self {
            AppConfigBoopConfInner::ODE(o) => BoopConfInner::ODE(o.to_livecode()),
            AppConfigBoopConfInner::Noop => BoopConfInner::Noop,
        }
    }
}

#[derive(Debug, Clone, Livecode)]
pub struct AppConfigFieldEntry {
    name: String,
    conf: AppConfigBoopConfInner,
}

fn _reset_b() -> ControlBool {
    ControlBool::force_from_str("kBf")
}

fn _base_noop_boop_conf() -> ControlAppConfigBoopConfInner {
    ControlAppConfigBoopConfInner::Noop
}

#[derive(Debug, Clone, Livecode)]
pub struct AppConfigBoopConf {
    #[livecode(serde_default = "_reset_b")]
    pub reset: bool, // if true, change immediately
    #[livecode(serde_default = "_base_noop_boop_conf")]
    base: AppConfigBoopConfInner,
    overrides: Vec<AppConfigFieldEntry>,
}
impl AppConfigBoopConf {
    fn to_livecode(&self) -> BoopConf {
        let fields = self
            .overrides
            .iter()
            .map(|x| (x.name.to_owned(), x.conf.to_livecode()))
            .collect();
        BoopConf::new(self.reset, self.base.to_livecode(), fields)
    }
}
impl Default for ControlAppConfigBoopConf {
    fn default() -> Self {
        ControlAppConfigBoopConf {
            reset: ControlBool::Raw(true),
            base: ControlAppConfigBoopConfInner::Noop,
            overrides: vec![],
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Livecode)]
pub struct SvgConfig {
    #[livecode(serde_default = "_default_svg_size")]
    pub size: f32,
    #[livecode(serde_default = "_default_svg_save")]
    pub save: bool, // trigger for svg save
}
impl Default for ControlSvgConfig {
    fn default() -> Self {
        Self {
            size: _default_svg_size(),
            save: _default_svg_save(),
        }
    }
}

fn _default_gpu_debug_next() -> ControlBool {
    ControlBool::force_from_str("kFf")
}

fn _default_gpu_debug() -> ControlBool {
    ControlBool::force_from_str("kDf")
}

fn _default_gpu_color_channel() -> ControlF32 {
    ControlF32::Int(0)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Livecode)]
pub struct GpuConfig {
    #[livecode(serde_default = "_default_gpu_debug_next")]
    debug_next: bool,
    #[livecode(serde_default = "_default_gpu_debug")]
    debug: bool, // trigger for svg save
    #[livecode(serde_default = "_default_gpu_color_channel")]
    color_channel: usize, // trigger for svg save
}
impl Default for ControlGpuConfig {
    fn default() -> Self {
        Self {
            debug_next: _default_gpu_debug_next(),
            debug: _default_gpu_debug(),
            color_channel: _default_gpu_color_channel(),
        }
    }
}

fn _default_should_reset() -> ControlBool {
    ControlBool::force_from_str("kVt")
}

#[allow(dead_code)]
#[derive(Debug, Clone, Livecode)]
pub struct AppConfig {
    #[livecode(serde_default = "_default_should_reset")]
    pub should_reset: bool, // should reset audio and time,
    #[livecode(serde_default = "false")]
    pub debug: bool,
    #[livecode(serde_default = "false")]
    pub capture: bool,
    #[livecode(serde_default = "_default_seed")]
    pub seed: f32,
    #[livecode(serde_default = "_default_width")]
    pub width: f32,
    #[livecode(serde_default = "_default_bg_alpha")]
    pub bg_alpha: f32,
    #[livecode(serde_default = "_default_clear_bg")]
    pub clear_bg: bool,
    #[livecode(serde_default = "_default_bg_color")]
    pub bg_color: MurreletColor,
    #[livecode(serde_default = "_default_capture_frame")]
    pub capture_frame: bool,
    #[livecode(serde_default = "1")]
    pub redraw: u64, // controls should_redraw, how many frames between redraw
    #[livecode(serde_default = "true")]
    pub reload: bool, // should reload and draw, good for slow drawing things
    #[livecode(serde_default = "0")]
    pub reload_rate: u64, // controls should_redraw, how many frames between redraw. if < 1, always defer to reload
    pub time: AppConfigTiming,
    #[livecode(kind = "none")]
    pub ctx: Node,
    #[livecode(serde_default = "default")]
    pub svg: SvgConfig,
    #[livecode(serde_default = "default")]
    pub gpu: GpuConfig,
    #[livecode(serde_default = "default")]
    pub boop: AppConfigBoopConf,
    // only reload on bar. this can be an easy way to sync visuals (e.g. only do big
    // changes when the bar hits), but can also slow down config changes if the bpm is low :o
    // so I usually disable this
    #[livecode(serde_default = "false")]
    pub reload_on_bar: bool,
}
impl AppConfig {
    pub fn should_clear_bg(&self) -> bool {
        self.bg_alpha > 0.5 || self.clear_bg
    }

    pub fn time(&self) -> LivecodeTimingConfig {
        self.time.to_livecode()
    }

    pub fn bg_alpha(&self) -> Option<f32> {
        if self.bg_alpha > 1e-5 {
            Some(self.bg_alpha)
        } else {
            None
        }
    }

    pub fn should_capture(&self) -> bool {
        self.capture_frame
    }

    fn should_reset(&self) -> bool {
        self.should_reset
    }

    fn reload_on_bar(&self) -> bool {
        self.reload_on_bar
    }
}

#[derive(Debug)]
struct BoopHolder<ConfType, BoopConfType>
where
    ConfType: ConfCommon<BoopConfType>,
    BoopConfType: BoopFromWorld<ConfType> + Clone,
{
    boop: BoopConfType,  // holds the state
    target: ConfType,    // holds the most recent target
    processed: ConfType, // holds the current locations
}

impl<ConfType, BoopConfType> BoopHolder<ConfType, BoopConfType>
where
    ConfType: ConfCommon<BoopConfType>,
    BoopConfType: BoopFromWorld<ConfType> + Clone,
{
    fn new(conf: &BoopConf, target: ConfType) -> Self {
        Self {
            boop: BoopConfType::boop_init(conf, &target),
            target: target.clone(),
            processed: target,
        }
    }

    fn update(&self, conf: &BoopConf, t: f32, target: ConfType) -> Self {
        let mut boop = self.boop.clone();
        let processed = boop.boop(conf, t, &target);
        Self {
            boop,
            target: target.clone(),
            processed,
        }
    }
}

#[derive(Debug)]
enum BoopMng<ConfType, BoopConfType>
where
    ConfType: ConfCommon<BoopConfType>,
    BoopConfType: BoopConfCommon<ConfType>,
{
    Uninitialized,
    NoBoop(ConfType),
    Boop(BoopHolder<ConfType, BoopConfType>),
}

impl<ConfType, BoopConfType> BoopMng<ConfType, BoopConfType>
where
    ConfType: ConfCommon<BoopConfType>,
    BoopConfType: BoopConfCommon<ConfType>,
{
    fn any_weird_states(&self) -> bool {
        match self {
            BoopMng::Uninitialized => false,
            BoopMng::NoBoop(_) => false,
            BoopMng::Boop(b) => b.boop.any_weird_states(),
        }
    }

    fn config(&self) -> &ConfType {
        match self {
            BoopMng::Uninitialized => unreachable!(),
            BoopMng::NoBoop(c) => c,
            BoopMng::Boop(c) => &c.processed,
        }
    }

    fn _reset(&self, target: ConfType) -> Self {
        // means we should set ourself to no-boop
        BoopMng::NoBoop(target)
    }

    fn _normal_update(&self, boop_conf: &BoopConf, t: f32, target: ConfType) -> Self {
        match self {
            BoopMng::Boop(b) => BoopMng::Boop(b.update(boop_conf, t, target)),
            _ => BoopMng::Boop(BoopHolder::new(boop_conf, target)),
        }
    }

    fn update(&self, boop_conf: &BoopConf, t: f32, target: ConfType) -> Self {
        if !boop_conf.reset() {
            self._normal_update(boop_conf, t, target)
        } else {
            self._reset(target)
        }
    }
}

pub struct LilLiveConfig<'a> {
    save_path: Option<&'a PathBuf>,
    run_id: u64,
    w: LiveCodeWorldState<'a>,
    app_config: &'a AppConfig,
}

pub fn svg_save_path(lil_liveconfig: &LilLiveConfig) -> SvgDrawConfig {
    svg_save_path_with_prefix(lil_liveconfig, "")
}

pub fn svg_save_path_with_prefix(lil_liveconfig: &LilLiveConfig, prefix: &str) -> SvgDrawConfig {
    let capture_path = if let Some(save_path) = lil_liveconfig.save_path {
        Some(capture_frame_name(
            save_path,
            lil_liveconfig.run_id,
            lil_liveconfig.w.time.actual_frame_u64(),
            prefix,
        ))
    } else {
        None
    };

    SvgDrawConfig::new(
        lil_liveconfig.app_config.width,
        capture_path,
        lil_liveconfig.app_config.svg.size,
        lil_liveconfig.w.time.actual_frame_u64(),
    )
}

fn capture_frame_name(save_path: &Path, run_id: u64, frame: u64, prefix: &str) -> PathBuf {
    let raw_name = format!("{}_capture_{:05}", run_id, { frame });
    let name = if !prefix.is_empty() {
        format!("{}_{}", prefix, raw_name)
    } else {
        raw_name
    };

    capture_folder(save_path, run_id).join(name)
}

fn capture_folder(save_path: &Path, run_id: u64) -> PathBuf {
    save_path.join(format!("{}", run_id))
}

pub struct LiveCoder<ConfType, ControlConfType, BoopConfType>
where
    ConfType: ConfCommon<BoopConfType>,
    BoopConfType: BoopConfCommon<ConfType>,
    ControlConfType: LiveCodeCommon<ConfType>,
{
    run_id: u64,
    controlconfig: ControlConfType,
    util: LiveCodeUtil,
    livecode_src: LivecodeSrc,
    save_path: Option<PathBuf>,
    prev_controlconfig: ControlConfType,
    boop_mng: BoopMng<ConfType, BoopConfType>,
    // sorry, the cache is mixed between boom_mng, but sometimes we need this
    cached_timeless_app_config: Option<AppConfig>,
}
impl<ConfType, ControlConfType, BoopConfType> LiveCoder<ConfType, ControlConfType, BoopConfType>
where
    ConfType: ConfCommon<BoopConfType>,
    BoopConfType: BoopConfCommon<ConfType>,
    ControlConfType: LiveCodeCommon<ConfType>,
{
    pub fn new_web(
        conf: String,
        livecode_src: LivecodeSrc,
    ) -> Result<LiveCoder<ConfType, ControlConfType, BoopConfType>, String> {
        let controlconfig = ControlConfType::parse(&conf);
        match controlconfig {
            Ok(c) => Ok(Self::new_full(c, None, livecode_src)),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn new(
        save_path: PathBuf,
        livecode_src: LivecodeSrc,
    ) -> LiveCoder<ConfType, ControlConfType, BoopConfType> {
        let controlconfig = ControlConfType::fs_load();
        Self::new_full(controlconfig, Some(save_path), livecode_src)
    }

    pub fn new_full(
        controlconfig: ControlConfType,
        save_path: Option<PathBuf>,
        livecode_src: LivecodeSrc,
    ) -> LiveCoder<ConfType, ControlConfType, BoopConfType> {
        let run_id = run_id();

        let util = LiveCodeUtil::new();

        let mut s = LiveCoder {
            run_id,
            controlconfig: controlconfig.clone(),
            livecode_src,
            util,
            save_path,
            prev_controlconfig: controlconfig,
            boop_mng: BoopMng::Uninitialized,
            cached_timeless_app_config: None, // uninitialized
        };

        // use the object to create a world and generate the configs
        s.set_processed_config();

        s
    }

    // experimental...
    pub fn set_control_config(&mut self, control_config: ControlConfType) {
        self.controlconfig = control_config;
    }

    // experimental...
    pub fn get_control_config(&self) -> ControlConfType {
        self.controlconfig.clone()
    }

    pub fn set_processed_config(&mut self) {
        // assume the target has already been updated
        // let target = self._extract_target_config();

        // set this one first, so we can use it to get the world
        self.cached_timeless_app_config =
            Some(self._app_config().just_midi(&self.timeless_world()));

        let w = self.world();

        let target = self.controlconfig.o(&w);

        let t = w.time.bar();

        let boop_conf = target.config_app_loc().boop.to_livecode();

        self.boop_mng = self.boop_mng.update(&boop_conf, t, target);
        if self.boop_mng.any_weird_states() {
            println!("some nans");
        }
    }

    pub fn svg_save_path(&self) -> SvgDrawConfig {
        self.svg_save_path_with_prefix("")
    }

    pub fn to_lil_liveconfig(&self) -> LilLiveConfig {
        LilLiveConfig {
            save_path: self.save_path.as_ref(),
            run_id: self.run_id,
            w: self.world(),
            app_config: self.app_config(),
        }
    }

    pub fn svg_save_path_with_prefix(&self, prefix: &str) -> SvgDrawConfig {
        svg_save_path_with_prefix(&self.to_lil_liveconfig(), prefix)
    }

    // sorry i'm near getting this to work so leaving this hacky and confusing
    // there's one for filesystems and one for callback..
    // filesystem one (watching folders)
    fn reload_config(&mut self) {
        if let Some(d) = ControlConfType::fs_load_if_needed_and_update_info(&mut self.util) {
            self.prev_controlconfig = self.controlconfig.clone();
            self.controlconfig = d;
        }
    }

    // web one, callback
    pub fn update_config_to(&mut self, text: &str) -> Result<(), String> {
        match ControlConfType::cb_reload_and_update_info(&mut self.util, text) {
            Ok(d) => {
                self.prev_controlconfig = self.controlconfig.clone();
                self.controlconfig = d;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// if the bg_alpha is above 0.5 or clear_bg is true
    pub fn should_reset_bg(&self) -> bool {
        self.world().time.actual_frame_u64() <= 1 || self.app_config().should_clear_bg()
    }

    pub fn maybe_bg_alpha(&self) -> Option<f32> {
        self.app_config().bg_alpha()
    }

    pub fn update(&mut self, app: &MurreletAppInput, reload: bool) {
        // use the previous frame's world for this
        let update_input = LivecodeSrcUpdateInput::new(
            self.app_config().debug,
            app,
            self.app_config().should_reset(),
        );

        self.livecode_src.update(&update_input);

        // needs to happen before checking is on bar
        self.util.update_with_frame(app.elapsed_frames());

        // if we can reload whenever, do that. otherwise only reload on bar

        if reload {
            if !self.app_config().reload_on_bar() || self.world().time.is_on_bar() {
                self.reload_config();
            }
        }

        if self.app_config().should_reset() {
            self.util.reset_time();
        }

        // needs to happen after checking is on bar
        self.util.update_last_render_time();

        // self.app_input.update(app);

        // this should happen at the very end
        self.set_processed_config();
    }

    pub fn timeless_world(&self) -> TimelessLiveCodeWorldState {
        self.util.timeless_world(
            // &self.midi.values, &self.app_input.values, &self.blte.values
            &self.livecode_src,
        )
    }

    pub fn world(&self) -> LiveCodeWorldState {
        // this function should only be called after this is set! since the "set processed" is called right away
        let timeless_app_config = self.cached_timeless_app_config.as_ref().unwrap();

        let timing_conf = timeless_app_config.time.to_livecode();
        let ctx = &timeless_app_config.ctx;

        self.util.world(&self.livecode_src, &timing_conf, ctx)
    }

    pub fn _app_config(&self) -> &ControlAppConfig {
        self.controlconfig._app_config()
    }

    pub fn app_config(&self) -> &AppConfig {
        // self._app_config().o(&self.world())
        // use the cached one
        self.config().config_app_loc()
    }

    pub fn gpu_color_channel(&self) -> usize {
        self.app_config().gpu.color_channel
    }

    pub fn config(&self) -> &ConfType {
        self.boop_mng.config()
    }

    // pub fn midi(&self) -> &MidiValues {
    //     &self.midi.values
    // }

    // pub fn audio(&self) -> &AudioValues {
    //     &self.audio.values
    // }

    // pub fn app_input_values(&self) -> &AppInputValues {
    //     &self.app_input.values
    // }

    // pub fn capture_folder(&self) -> PathBuf {
    //     capture_folder(self.save_path(), self.run_id)
    // }

    pub fn capture_frame_name(&self, frame: u64, prefix: &str) -> Option<PathBuf> {
        if let Some(save_path) = &self.save_path {
            Some(capture_frame_name(&save_path, self.run_id, frame, prefix))
        } else {
            None
        }
    }

    // model.livecode.capture_logic(|img_name: PathBuf| { app.main_window().capture_frame(img_name); } );
    pub fn capture<F>(&self, capture_frame_fn: F)
    where
        F: Fn(PathBuf) -> (),
    {
        let frame = self.world().actual_frame_u64();

        if let Some(capture_frame_name) = self.capture_frame_name(frame, "") {
            let img_name = capture_frame_name.with_extension("png");
            println!("writing to {:?}", img_name);

            capture_frame_fn(img_name);

            // save a copy of the config
            if !self.app_config().capture {
                let img_name = capture_frame_name.with_extension("txt");
                fs::write(img_name, format!("{:?}", self.config())).expect("Unable to write file");
                let img_name = capture_frame_name.with_extension("yaml");
                fs::copy(env::args().collect::<Vec<String>>()[1].clone(), img_name).unwrap();
            }
        }
    }

    pub fn capture_with_fn<F>(&self, capture_frame_fn: F)
    where
        F: Fn(PathBuf) -> (),
    {
        let w = self.world();

        let frame = w.actual_frame_u64();
        if (self.app_config().capture && frame != 0) || self.app_config().should_capture() {
            let frame_freq = 1;
            if frame % frame_freq == 0 {
                self.capture(capture_frame_fn);
            }
        }
    }

    pub fn was_updated(&self) -> bool {
        self.util.updated()
    }

    // could i make this also work on when the config is fresh?
    pub fn should_reload(&self) -> bool {
        let w = self.world();
        let reload_rate = self.app_config().reload_rate;
        let reload_rate_says_so =
            reload_rate >= 1 && w.actual_frame() as u64 % self.app_config().reload_rate == 0;
        let config_says_so = self.app_config().reload;
        reload_rate_says_so || config_says_so
    }

    pub fn should_redraw(&self) -> bool {
        let w = self.world();
        let redraw_says_so = w.actual_frame() as u64 % self.app_config().redraw == 0;
        let save_says_so = self.app_config().svg.save;
        // might have other things..
        redraw_says_so || save_says_so
    }

    pub fn should_save_svg(&self) -> bool {
        self.app_config().svg.save
    }

    pub fn should_show_gpu_debug(&self) -> bool {
        self.app_config().gpu.debug
    }

    pub fn should_show_next_gpu_debug(&self) -> bool {
        self.app_config().gpu.debug_next
    }

    pub fn save_path(&self) -> Option<&PathBuf> {
        self.save_path.as_ref()
    }

    pub fn frame(&self) -> u64 {
        self.world().actual_frame() as u64
    }

    // seconds since last render
    pub fn time_delta(&self) -> f32 {
        self.world().time.seconds_between_render_times()
    }
}
