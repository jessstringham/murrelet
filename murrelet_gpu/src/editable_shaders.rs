use std::collections::HashMap;

use crate::{
    build_shader_custom_vertex, gpu_macros::ShaderStr, graphics_ref::GraphicsVertex,
    window::GraphicsWindowConf,
};
use lerpable::Lerpable;
use murrelet_common::triangulate::DefaultVertex;
use murrelet_livecode::types::LivecodeResult;
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

        let shader_changed_and_compiles = if !self.has_changed(prev) {
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
