#![allow(dead_code)]
use glam::{vec3, Mat4, Vec2};
use lerpable::Lerpable;
use murrelet_common::{Assets, AssetsRef, LivecodeUsage};
use murrelet_common::{LivecodeSrc, LivecodeSrcUpdateInput, MurreletAppInput};
use murrelet_common::{MurreletColor, TransformVec2};
use murrelet_gui::MurreletGUI;
use murrelet_livecode::expr::MixedEvalDefs;
use murrelet_livecode::lazy::ControlLazyNodeF32;
use murrelet_livecode::state::{LivecodeTimingConfig, LivecodeWorldState};
use murrelet_livecode::types::{AdditionalContextNode, ControlVecElement, LivecodeResult};
use std::collections::{HashMap, HashSet};
use std::fs;

use murrelet_common::run_id;
use std::path::{Path, PathBuf};

use murrelet_livecode::livecode::LivecodeFromWorld;
use murrelet_livecode::livecode::*;
use murrelet_livecode_derive::Livecode;

use crate::asset_loader::*;
use crate::cli::{BaseConfigArgs, TextureDimensions};
use crate::reload::*;
use clap::Parser;

pub trait CommonTrait: std::fmt::Debug + Clone {}

// requirements for the control conf
pub trait LiveCodeCommon<T>:
    GetLivecodeIdentifiers + LivecodeFromWorld<T> + LiveCoderLoader + CommonTrait
{
}

// requirements for the conf
pub trait ConfCommon: CommonTrait {
    fn config_app_loc(&self) -> &AppConfig;
}

#[derive(Clone, Debug)]
pub struct SvgDrawConfig {
    size: f32, // todo, what's the difference between this and texture sizes?
    pub resolution: Option<TextureDimensions>,
    capture_path: Option<PathBuf>, // if it's missing, we don't save (e.g. web browser)
    frame: u64,
    target_size: f32, // in mm
    margin_size: f32,
    should_resize: bool, // sorry, something to force it not to resize my shapes on the web!
    bg_color: Option<MurreletColor>,
}
impl SvgDrawConfig {
    pub fn new(
        size: f32,
        resolution: Option<TextureDimensions>,
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
            should_resize: true,
            resolution,
            bg_color: None
        }
    }

    pub fn with_bg_color(&self, bg_color: MurreletColor) -> Self {
        let mut c = self.clone();
        c.bg_color = Some(bg_color);
        c
    }

    pub fn with_no_resize(&self) -> Self {
        let mut c = self.clone();
        c.should_resize = false;
        c
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
        if self.should_resize {
            // okay so we take the width, since that's what looked okay on the screen
            let size = self.size();
            let full_target_width = self.full_target_width() * 1.0;

            let translation_to_final = vec3(full_target_width, full_target_width, 0.0);
            let s = self.target_size / size;
            let scale = vec3(s, s, 1.0);

            // aiming for 100mm by 100mm, going from 0 to 10
            // operations go right to left!
            Mat4::from_translation(translation_to_final) * Mat4::from_scale(scale)
        } else {
            Mat4::IDENTITY
        }
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn bg_color(&self) -> Option<MurreletColor> {
        self.bg_color
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

fn _default_seed_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(42.0)
}
fn _default_width_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(400.0)
}

fn _default_bpm() -> ControlF32 {
    ControlF32::Raw(90.0)
}
fn _default_bpm_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(90.0)
}
fn _default_fps() -> ControlF32 {
    ControlF32::Raw(30.0)
}
fn _default_fps_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(30.0)
}
fn _default_beats_per_bar() -> ControlF32 {
    ControlF32::Raw(4.0)
}
fn _default_beats_per_bar_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(4.0)
}

fn _default_bg_alpha() -> ControlF32 {
    #[cfg(feature = "for_the_web")]
    {
        ControlF32::Raw(1.0) // also not really used on the web atm
    }
    #[cfg(not(feature = "for_the_web"))]
    {
        ControlF32::force_from_str("slog(m15, -5.0, 0.0)")
    }
}

fn _default_bg_alpha_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(1.0)
}

fn _default_capture_frame() -> ControlBool {
    #[cfg(feature = "for_the_web")]
    {
        ControlBool::Raw(false) // can't actually capture the frame..
    }
    #[cfg(not(feature = "for_the_web"))]
    {
        ControlBool::force_from_str("kSf")
    }
} // usually want to leave this as midi

fn _default_capture_frame_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Bool(false)
}

fn _default_clear_bg() -> ControlBool {
    #[cfg(feature = "for_the_web")]
    {
        ControlBool::Raw(false)
    }
    #[cfg(not(feature = "for_the_web"))]
    {
        ControlBool::force_from_str("m12") // todo, make this relax if missing
    }
} // usually want to leave this as midi

fn _default_clear_bg_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Bool(true)
}

fn _default_bg_color() -> [ControlF32; 4] {
    [
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(1.0),
    ]
}

fn _default_bg_color_lazy() -> Vec<ControlVecElement<ControlLazyNodeF32>> {
    vec![
        ControlVecElement::raw(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(1.0)),
    ]
}

fn _default_svg_size() -> ControlF32 {
    ControlF32::Raw(100.0)
}
fn _default_svg_save() -> ControlBool {
    ControlBool::Raw(false)
}

fn _default_svg_size_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(100.0)
}
fn _default_svg_save_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Bool(false)
}

// this stuff adjusts how time works, so needs to be split off pretty early
#[allow(dead_code)]
#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
pub struct AppConfigTiming {
    #[livecode(serde_default = "_default_bpm")]
    pub bpm: f32,
    #[livecode(serde_default = "_default_beats_per_bar")]
    pub beats_per_bar: f32,
    #[livecode(serde_default = "_default_fps")]
    pub fps: f32,
    #[livecode(serde_default = "true")]
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

fn _reset_b() -> ControlBool {
    ControlBool::force_from_str("kBf")
}

fn _reset_b_lazy() -> ControlLazyNodeF32 {
    // ControlLazyNodeF32::from_control_bool(false)
    unimplemented!("no lazy bools yet??")
}

#[allow(dead_code)]
#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
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
impl Default for ControlLazySvgConfig {
    fn default() -> Self {
        Self {
            size: _default_svg_size_lazy(),
            save: _default_svg_save_lazy(),
        }
    }
}

fn _default_gpu_debug_next() -> ControlBool {
    #[cfg(feature = "for_the_web")]
    {
        ControlBool::Raw(false)
    }
    #[cfg(not(feature = "for_the_web"))]
    {
        ControlBool::force_from_str("kFf")
    }
}

fn _default_gpu_debug() -> ControlBool {
    #[cfg(feature = "for_the_web")]
    {
        ControlBool::Raw(false)
    }
    #[cfg(not(feature = "for_the_web"))]
    {
        ControlBool::force_from_str("kDf")
    }
}

fn _default_gpu_color_channel() -> ControlF32 {
    ControlF32::Int(0)
}

fn _default_gpu_debug_next_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Bool(false)
}

fn _default_gpu_debug_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Bool(false)
}

fn _default_gpu_color_channel_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Int(0)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
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

impl Default for ControlLazyGpuConfig {
    fn default() -> Self {
        Self {
            debug_next: _default_gpu_debug_next_lazy(),
            debug: _default_gpu_debug_lazy(),
            color_channel: _default_gpu_color_channel_lazy(),
        }
    }
}

fn _default_should_reset() -> ControlBool {
    #[cfg(feature = "for_the_web")]
    {
        ControlBool::Raw(false)
    }
    #[cfg(not(feature = "for_the_web"))]
    {
        ControlBool::force_from_str("kVt")
    }
}

fn _default_should_reset_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Bool(false)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
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
    pub ctx: AdditionalContextNode,
    #[livecode(serde_default = "default")]
    pub svg: SvgConfig,
    #[livecode(serde_default = "default")]
    pub gpu: GpuConfig,
    // only reload on bar. this can be an easy way to sync visuals (e.g. only do big
    // changes when the bar hits), but can also slow down config changes if the bpm is low :o
    // so I usually disable this
    #[livecode(serde_default = "false")]
    pub reload_on_bar: bool,
    #[livecode(serde_default = "_empty_filenames")]
    pub assets: AssetFilenames, // for svg files!
    #[livecode(serde_default = "0")] // if 0, it won't run at all
    pub lerp_rate: f32,
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

    fn should_lerp(&self) -> bool {
        self.lerp_rate > 0.0
    }
}

// todo, this is all a little weird (svg save path), i should revisit it..
pub struct LilLiveConfig<'a> {
    resolution: Option<TextureDimensions>,
    save_path: Option<&'a PathBuf>,
    run_id: u64,
    w: &'a LivecodeWorldState,
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
            lil_liveconfig.w.actual_frame_u64(),
            prefix,
        ))
    } else {
        None
    };

    SvgDrawConfig::new(
        lil_liveconfig.app_config.width,
        lil_liveconfig.resolution,
        capture_path,
        lil_liveconfig.app_config.svg.size,
        lil_liveconfig.w.actual_frame_u64(),
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

pub struct LiveCoder<ConfType, ControlConfType>
where
    ConfType: ConfCommon + Send + Sync,
    ControlConfType: LiveCodeCommon<ConfType>,
{
    run_id: u64,
    pub controlconfig: ControlConfType,            // latest one
    queued_configcontrol: Option<ControlConfType>, // if a new one comes in before we're done, queue it!
    util: LiveCodeUtil,
    livecode_src: LivecodeSrc, // get info from outside world
    save_path: Option<PathBuf>,
    pub prev_controlconfig: ControlConfType, // last one
    curr_conf: Option<ConfType>,
    // sorry, the cache is mixed between curr_conf, but sometimes we need this
    cached_timeless_app_config: Option<AppConfigTiming>,
    cached_world: Option<LivecodeWorldState>,
    assets: AssetsRef,
    maybe_args: Option<BaseConfigArgs>, // should redesign this...
    lerp_pct: f32,                      // moving between things
    used_variable_names: HashSet<String>,
}
impl<ConfType, ControlConfType> LiveCoder<ConfType, ControlConfType>
where
    ConfType: ConfCommon + Send + Sync + Lerpable,
    ControlConfType: LiveCodeCommon<ConfType>,
{
    pub fn new_web(
        conf: String,
        livecode_src: LivecodeSrc,
        load_funcs: &AssetLoaders,
    ) -> LivecodeResult<LiveCoder<ConfType, ControlConfType>> {
        let controlconfig = ControlConfType::parse(&conf)?;
        // .map_err(|err| {
        // if let Some(error) = err.location() {
        //     LivecodeError::SerdeLoc(error, err.to_string())
        // } else {
        //     LivecodeError::Raw(err.to_string())
        // }
        // })?;
        Self::new_full(controlconfig, None, livecode_src, load_funcs, None)
    }

    // this one panics if something goes wrong
    pub fn new(
        save_path: PathBuf,
        livecode_src: LivecodeSrc,
        load_funcs: &AssetLoaders,
    ) -> LiveCoder<ConfType, ControlConfType> {
        let controlconfig = ControlConfType::fs_load();

        let args = BaseConfigArgs::parse();

        let result = Self::new_full(
            controlconfig,
            Some(save_path),
            livecode_src,
            load_funcs,
            Some(args),
        );
        result.expect("error loading!")
    }

    pub fn new_full(
        controlconfig: ControlConfType,
        save_path: Option<PathBuf>,
        livecode_src: LivecodeSrc,
        load_funcs: &AssetLoaders,
        maybe_args: Option<BaseConfigArgs>,
    ) -> LivecodeResult<LiveCoder<ConfType, ControlConfType>> {
        let run_id = run_id();

        let util = LiveCodeUtil::new()?;

        let used_variable_names = controlconfig
            .variable_identifiers()
            .into_iter()
            .map(|x| x.name)
            .collect();

        let mut s = LiveCoder {
            run_id,
            controlconfig: controlconfig.clone(),
            queued_configcontrol: None,
            livecode_src,
            util,
            save_path,
            prev_controlconfig: controlconfig,
            curr_conf: None,
            cached_timeless_app_config: None, // uninitialized
            cached_world: None,
            assets: Assets::empty_ref(),
            maybe_args,
            lerp_pct: 1.0, // start in the done state!
            used_variable_names,
        };

        // hrm, before doing most things, load the assets (but we'll do this line again...)

        s.cached_timeless_app_config = Some(s._timing_config().o(&s._timeless_world()?)?);
        s._update_world()?;

        let w = s.world();
        let app_conf = s.controlconfig._app_config().o(w)?;
        let assets = app_conf.assets.load_polylines(load_funcs);
        s.assets = assets.to_ref();

        // use the object to create a world and generate the configs
        s.set_processed_config()?;

        Ok(s)
    }

    // experimental...
    pub fn set_control_config(&mut self, control_config: ControlConfType) {
        self.controlconfig = control_config;
    }

    // experimental...
    pub fn get_control_config(&self) -> ControlConfType {
        self.controlconfig.clone()
    }

    // if there are any issues at this point with the config, it'll bail and
    // not update the config.
    // in the case of updating live, you might just print the result but keep
    // going with the existing config until you fix it.
    // with initially loading it, you might just not start the program
    pub fn set_processed_config(&mut self) -> LivecodeResult<()> {
        // set this one first, so we can use it to get the world

        self.cached_timeless_app_config = Some(self._timing_config().o(&self._timeless_world()?)?);
        self._update_world()?;

        let w = self.world();

        let mut target = self.controlconfig.o(&w)?;
        let mut lerp_change = 0.0;

        // todo, make this optional
        if target.config_app_loc().should_lerp() {
            if self.lerp_pct < 1.0 {
                let old_target = self.prev_controlconfig.o(&w)?;
                target = old_target.lerpify(&target, &self.lerp_pct);
            }

            // prepare this for next time
            lerp_change = self.time_delta() * target.config_app_loc().lerp_rate;
        };

        let _t = w.time().bar();

        // set the current config
        self.curr_conf = Some(target);

        self.lerp_pct += lerp_change;

        if self.lerp_pct >= 1.0 {
            if let Some(new_target) = &self.queued_configcontrol {
                self.prev_controlconfig = self.controlconfig.clone();
                self.controlconfig = new_target.clone();
                self.lerp_pct = 0.0;
                self.queued_configcontrol = None;
            }
        }

        Ok(())
    }

    pub fn svg_save_path(&self) -> SvgDrawConfig {
        self.svg_save_path_with_prefix("")
    }

    pub fn to_lil_liveconfig(&self) -> LivecodeResult<LilLiveConfig> {
        Ok(LilLiveConfig {
            save_path: self.save_path.as_ref(),
            run_id: self.run_id,
            w: self.world(),
            app_config: self.app_config(),
            resolution: self.maybe_args.as_ref().map(|x| x.resolution),
        })
    }

    pub fn svg_save_path_with_prefix(&self, prefix: &str) -> SvgDrawConfig {
        // unwrapping here, should check if this could fail
        svg_save_path_with_prefix(&self.to_lil_liveconfig().unwrap(), prefix)
    }

    // sorry i'm near getting this to work so leaving this hacky and confusing
    // there's one for filesystems and one for callback..
    // filesystem one (watching folders)
    fn reload_config(&mut self) {
        let result = ControlConfType::fs_load_if_needed_and_update_info(&mut self.util);
        if let Ok(Some(d)) = result {
            // if we're in the middle of something, put this in the queue
            if self.lerp_pct < 1.0 && self.lerp_pct > 0.0 {
                self.queued_configcontrol = Some(d);
            } else {
                self.prev_controlconfig = self.controlconfig.clone();
                self.controlconfig = d;
                self.lerp_pct = 0.0; // reloaded, so time to reload it!
            }

            // set the current vars
            let variables_iter = self
                .controlconfig
                .variable_identifiers()
                .into_iter()
                .chain(self.prev_controlconfig.variable_identifiers().into_iter());
            let variables = if let Some(queued) = &self.queued_configcontrol {
                variables_iter
                    .chain(queued.variable_identifiers().into_iter())
                    .map(|x| x.name)
                    .collect::<HashSet<String>>()
            } else {
                variables_iter.map(|x| x.name).collect::<HashSet<String>>()
            };
            self.used_variable_names = variables;
        } else if let Err(e) = result {
            eprintln!("Error {}", e);
        }
    }

    // web one, callback
    pub fn update_config_to(&mut self, text: &str) -> Result<(), String> {
        match ControlConfType::cb_reload_and_update_info(&mut self.util, text) {
            Ok(d) => {
                self.prev_controlconfig = self.controlconfig.clone();
                self.controlconfig = d;
                self.queued_configcontrol = None;
                self.lerp_pct = 0.0;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// if the bg_alpha is above 0.5 or clear_bg is true
    pub fn should_reset_bg(&self) -> bool {
        self.world().actual_frame_u64() <= 1 || self.app_config().should_clear_bg()
    }

    pub fn maybe_bg_alpha(&self) -> Option<f32> {
        self.app_config().bg_alpha()
    }

    // called every frame
    pub fn update(&mut self, app: &MurreletAppInput, reload: bool) -> LivecodeResult<()> {
        // use the previous frame's world for this
        let update_input = LivecodeSrcUpdateInput::new(
            self.app_config().debug,
            app,
            self.app_config().should_reset(),
        );

        self.livecode_src.update(&update_input);

        if app.elapsed_frames() % 20 == 0 {
            let variables = self
                .controlconfig
                .variable_identifiers()
                .into_iter()
                .map(|x| {
                    (
                        x.name.clone(),
                        LivecodeUsage {
                            name: x.name.clone(),
                            is_used: true,
                            value: None, // todo
                        },
                    )
                })
                .collect::<HashMap<_, _>>();

            self.livecode_src.feedback(&variables);
        }

        // needs to happen before checking is on bar
        self.util.update_with_frame(app.elapsed_frames());

        // if we can reload whenever, do that. otherwise only reload on bar

        if reload {
            if !self.app_config().reload_on_bar() || self.world().time().is_on_bar() {
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
        // cache the world
        self.set_processed_config()
    }

    pub fn _timeless_world(&self) -> LivecodeResult<LivecodeWorldState> {
        self.util.timeless_world(&self.livecode_src)
    }

    pub fn _update_world(&mut self) -> LivecodeResult<()> {
        // this function should only be called after this is set! since the "set processed" is called right away
        let timeless_app_config = self.cached_timeless_app_config.as_ref().unwrap();
        let timing_conf = timeless_app_config.to_livecode();

        let ctx = &self.controlconfig._app_config().ctx;

        let mut world =
            self.util
                .world(&self.livecode_src, &timing_conf, ctx, self.assets.clone())?;

        let mut md = MixedEvalDefs::new();

        for x in self.used_variable_names.difference(&world.vars()) {
            // argh, so used_variable_names includes non-global things, but right now i'm global
            // so just do the stuff I care about right now, osc things...
            if x.starts_with("oo_") {
                if world.actual_frame_u64() % 1000 == 0 {
                    println!("adding default value for {:?}", x);
                }
                md.set_val(x, murrelet_common::LivecodeValue::Float(0.0));
            }
        }
        world.update_with_defs(&md).unwrap(); // i'm setting this so it should be okay..

        self.cached_world = Some(world);
        Ok(())
    }

    pub fn world(&self) -> &LivecodeWorldState {
        self.cached_world.as_ref().unwrap()
    }

    pub fn _timing_config(&self) -> &ControlAppConfigTiming {
        &self.controlconfig._app_config().time
    }

    pub fn app_config(&self) -> &AppConfig {
        // use the cached one
        self.config().config_app_loc()
    }

    pub fn gpu_color_channel(&self) -> usize {
        self.app_config().gpu.color_channel
    }

    pub fn config(&self) -> &ConfType {
        self.curr_conf.as_ref().unwrap() // should be set
    }

    pub fn capture_frame_name(&self, frame: u64, prefix: &str) -> Option<PathBuf> {
        if let Some(save_path) = &self.save_path {
            Some(capture_frame_name(&save_path, self.run_id, frame, prefix))
        } else {
            None
        }
    }

    // model.livecode.capture_logic(|img_name: PathBuf| { app.main_window().capture_frame(img_name); } );
    pub fn capture<F>(&self, capture_frame_fn: F) -> LivecodeResult<()>
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

                if let Some(env) = &self.maybe_args {
                    fs::copy(env.config_path.clone(), img_name).unwrap();
                } else {
                    println!("Hm, didn't have a base config args, but trying to save...");
                }
            }
        }
        Ok(())
    }

    pub fn capture_with_fn<F>(&self, capture_frame_fn: F) -> LivecodeResult<()>
    where
        F: Fn(PathBuf) -> (),
    {
        let w = self.world();

        let should_capture = self
            .maybe_args
            .as_ref()
            .map(|env| env.capture)
            .unwrap_or(false);

        let frame = w.actual_frame_u64();
        if (self.app_config().capture && frame != 0)
            || self.app_config().should_capture()
            || should_capture
        {
            let frame_freq = 1;
            if frame % frame_freq == 0 {
                self.capture(capture_frame_fn)?;
            }
        }

        Ok(())
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
        self.world().time().seconds_between_render_times()
    }

    pub fn args(&self) -> BaseConfigArgs {
        self.maybe_args.clone().unwrap()
    }

    pub fn sketch_args(&self) -> Vec<String> {
        if let Some(args) = &self.maybe_args {
            args.sketch_args.clone()
        } else {
            vec![]
        }
    }
}
