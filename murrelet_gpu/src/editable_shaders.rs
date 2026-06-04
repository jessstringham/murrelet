use std::collections::HashMap;

use crate::{
    build_shader_custom_vertex, gpu_macros::ShaderStr, graphics_ref::GraphicsVertex,
    window::GraphicsWindowConf,
};
use lerpable::Lerpable;
use murrelet_common::triangulate::DefaultVertex;
use murrelet_livecode::types::{LivecodeError, LivecodeResult};
use murrelet_livecode_derive::Livecode;
use naga;
#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

use crate::{
    build_shader, build_shader_2tex,
    graphics_ref::{GraphicsCreator, GraphicsRefCustom},
};

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct ShaderStrings {
    #[livecode(kind = "none")]
    #[lerpable(method = "skip")]
    shaders: HashMap<String, String>,
}
impl ShaderStrings {
    // fn shader_str<VertexKind: GraphicsVertex>(shader: &str) -> String {
    //     Self::shader_custom_prefix(shader, VertexKind::fragment_prefix())
    // }

    fn shader(shader: &str) -> String {
        build_shader! {
            (
                raw shader;
            )
        }
    }

    fn shader_custom_prefix(shader: &str, prefix: &str) -> String {
        build_shader_custom_vertex! {
            (
                prefix prefix;
                raw shader;
            )
        }
    }

    fn shader2tex(shader: &str) -> String {
        build_shader_2tex! {
            (
                raw shader;
            )
        }
    }

    pub fn get_shader_str(&self, _c: &GraphicsWindowConf, name: &str) -> Option<String> {
        self.shaders.get(name).map(|str| Self::shader(str))
    }

    pub fn get_shader(&self, name: &str) -> Option<String> {
        self.shaders.get(name).cloned()
    }

    pub fn get_shader_str_2tex(&self, _c: &GraphicsWindowConf, name: &str) -> Option<String> {
        self.shaders.get(name).map(|str| Self::shader2tex(str))
    }

    pub fn get_shader_str_custom_prefix(
        &self,
        _c: &GraphicsWindowConf,
        name: &str,
        prefix: &str,
    ) -> Option<String> {
        self.shaders
            .get(name)
            .map(|str| Self::shader_custom_prefix(str, prefix))
    }

    pub fn get_graphics_ref(
        &self,
        c: &GraphicsWindowConf,
        name: &str,
    ) -> Option<GraphicsRefCustom<DefaultVertex>> {
        self.shaders.get(name).map(|str| {
            GraphicsCreator::<DefaultVertex>::default()
                .with_mag_filter(wgpu::FilterMode::Nearest)
                .to_graphics_ref(c, name, &Self::shader(str))
        })
    }

    pub fn get_graphics_ref_2tex(
        &self,
        c: &GraphicsWindowConf,
        name: &str,
    ) -> Option<GraphicsRefCustom<DefaultVertex>> {
        self.shaders.get(name).map(|str| {
            GraphicsCreator::<DefaultVertex>::default()
                .with_mag_filter(wgpu::FilterMode::Nearest)
                .with_second_texture()
                .to_graphics_ref(c, name, &Self::shader2tex(str))
        })
    }
}

impl ControlShaderStrings {
    fn to_normal(&self) -> ShaderStrings {
        ShaderStrings {
            shaders: self.shaders.clone(),
        }
    }

    pub fn has_changed(&self, other: &ControlShaderStrings) -> bool {
        self.shaders != other.shaders
    }

    pub fn should_update<VertexKind: GraphicsVertex, CustomShaderStrings: CustomShaderString>(
        &self,
        prev: &ControlShaderStrings,
        force_reload: bool,
    ) -> Option<CustomShaderStrings> {
        let custom_shader = CustomShaderStrings::from_ctrl_shader_str(self).ok()?;

        let shader_changed_and_compiles = if self.has_changed(prev) {
            custom_shader.naga_if_needed::<VertexKind>()
        } else {
            false
        };

        if force_reload || shader_changed_and_compiles {
            // just in case there's lerp, be sure to use the one we tested
            Some(custom_shader)
        } else {
            None
        }
    }
}

pub trait CustomShaderString: Sized {
    fn from_shader_str(c: &ShaderStrings) -> LivecodeResult<Self>;

    fn built_shaders_for_validation(&self) -> Vec<(&str, String)>;

    fn from_ctrl_shader_str(c: &ControlShaderStrings) -> LivecodeResult<Self> {
        Self::from_shader_str(&c.to_normal())
    }

    fn naga_if_needed<VertexKind: GraphicsVertex>(&self) -> bool {
        let mut all_success = true;

        for (name, shader_str) in self.built_shaders_for_validation().iter() {
            if let Err(err) = naga::front::wgsl::parse_str(shader_str) {
                println!(
                    "error with shader {:?}, {:?}, not updating until it works!",
                    name, err
                );
                all_success = false;
            }
        }

        all_success
    }

    fn to_2tex_graphics(
        c: &GraphicsWindowConf,
        name: &str,
        shader_str: &str,
    ) -> GraphicsRefCustom<DefaultVertex> {
        GraphicsCreator::<DefaultVertex>::default()
            .with_mag_filter(wgpu::FilterMode::Nearest)
            .with_second_texture()
            .to_graphics_ref(c, name, shader_str)
    }

    fn shader(shader: &str) -> String {
        build_shader! {
            (
                raw shader;
            )
        }
    }

    fn shader_custom_prefix(shader: &str, prefix: &str) -> String {
        build_shader_custom_vertex! {
            (
                prefix prefix;
                raw shader;
            )
        }
    }

    fn shader2tex(shader: &str) -> String {
        build_shader_2tex! {
            (
                raw shader;
            )
        }
    }
}

// State holder for a sketch's live-uploadable shader(s). Owns the currently-built
// shaders (for change detection) plus a `needs_init` flag the windowed harness reads
// to force one feedback re-seed after a full (re)build. A live shader edit is applied
// in place via `GraphicsRefCustom::update_shader` (keeping the feedback textures), so
// the accumulation continues instead of flashing; a full rebuild is the opt-in clear.
pub struct CustomShaderState<S: CustomShaderString> {
    built: S,
    needs_init: std::cell::Cell<bool>,
}

impl<S: CustomShaderString + PartialEq> CustomShaderState<S> {
    pub fn new(built: S) -> Self {
        Self {
            built,
            needs_init: std::cell::Cell::new(true),
        }
    }

    pub fn built(&self) -> &S {
        &self.built
    }

    // Read-and-clear the "freshly (re)built, re-seed feedback next frame" flag. The
    // windowed harness ORs this into `global_reset` in `render_in`.
    pub fn take_needs_init(&self) -> bool {
        self.needs_init.replace(false)
    }

    // Mark that the feedback should be re-seeded next frame (the opt-in clear).
    pub fn mark_needs_init(&self) {
        self.needs_init.set(true);
    }

    // If `candidate` differs from the current built shaders AND parses (naga), hand it
    // to `apply` — the sketch hot-swaps each shader into its pipeline via
    // `GraphicsRefCustom::update_shader` — then record it as the new built. Returns
    // whether a swap happened. A shader that doesn't parse keeps the last good one on
    // screen (naga prints the error; `built` is left untouched).
    pub fn swap_if_changed<VertexKind: GraphicsVertex>(
        &mut self,
        candidate: S,
        apply: impl FnOnce(&S),
    ) -> bool {
        if candidate == self.built {
            return false;
        }
        if !candidate.naga_if_needed::<VertexKind>() {
            return false;
        }
        apply(&candidate);
        self.built = candidate;
        true
    }

    // Build the candidate from config `ShaderStrings` first (a missing shader keeps the
    // old one), then `swap_if_changed`.
    pub fn swap_if_changed_from<VertexKind: GraphicsVertex>(
        &mut self,
        shaders: &ShaderStrings,
        apply: impl FnOnce(&S),
    ) -> bool {
        match S::from_shader_str(shaders) {
            Ok(candidate) => self.swap_if_changed::<VertexKind>(candidate, apply),
            Err(_) => false,
        }
    }

    // For sketches that rebuild the WHOLE graphic on a shader change instead of
    // hot-swapping in place (e.g. when a shader drives a compute pipeline that has no
    // in-place swap, and there's no feedback accumulation to preserve): a non-mutating
    // predicate for the `needs_rebuild` hook. True when `force` or the candidate
    // differs from the current built shaders, AND it parses (naga) — so a typo keeps
    // the last good graphic. The rebuilt graphic constructs a fresh state.
    pub fn should_full_rebuild<VertexKind: GraphicsVertex>(
        &self,
        candidate: &S,
        force: bool,
    ) -> bool {
        if !(force || candidate != &self.built) {
            return false;
        }
        candidate.naga_if_needed::<VertexKind>()
    }
}

// Shortcut `CustomShaderString` for a sketch with a single fragment shader (the
// `build_shader!` form), so it doesn't have to hand-write a strings struct. Holds the
// BUILT shader string; build it from raw config text with `SingleShader::build`.
#[derive(Clone, PartialEq)]
pub struct SingleShader(String);
impl SingleShader {
    pub fn build(raw: &str) -> Self {
        SingleShader(<Self as CustomShaderString>::shader(raw))
    }
    pub fn built_str(&self) -> &str {
        &self.0
    }
}
impl CustomShaderString for SingleShader {
    fn from_shader_str(c: &ShaderStrings) -> LivecodeResult<Self> {
        let s = c
            .get_shader("shader")
            .ok_or(LivecodeError::raw("missing shader"))?;
        Ok(SingleShader::build(&s))
    }
    fn built_shaders_for_validation(&self) -> Vec<(&str, String)> {
        vec![("shader", self.0.clone())]
    }
}

// Single-shader shortcut for the two-texture (`build_shader_2tex!`) form.
#[derive(Clone, PartialEq)]
pub struct SingleShader2Tex(String);
impl SingleShader2Tex {
    pub fn build(raw: &str) -> Self {
        SingleShader2Tex(<Self as CustomShaderString>::shader2tex(raw))
    }
    pub fn built_str(&self) -> &str {
        &self.0
    }
}
impl CustomShaderString for SingleShader2Tex {
    fn from_shader_str(c: &ShaderStrings) -> LivecodeResult<Self> {
        let s = c
            .get_shader("shader")
            .ok_or(LivecodeError::raw("missing shader"))?;
        Ok(SingleShader2Tex::build(&s))
    }
    fn built_shaders_for_validation(&self) -> Vec<(&str, String)> {
        vec![("shader", self.0.clone())]
    }
}
