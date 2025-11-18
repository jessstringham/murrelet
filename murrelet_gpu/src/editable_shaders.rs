use std::collections::HashMap;

use crate::{
    build_shader_custom_vertex, gpu_macros::ShaderStr, graphics_ref::GraphicsVertex,
    window::GraphicsWindowConf,
};
use lerpable::Lerpable;
use murrelet_common::triangulate::DefaultVertex;
use murrelet_livecode_derive::Livecode;
use wgpu_for_latest::naga;
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
    fn shader_str<VertexKind: GraphicsVertex>(shader: &str) -> String {
        Self::shader_custom_prefix(shader, VertexKind::fragment_prefix())
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

    pub fn get_shader_str(&self, _c: &GraphicsWindowConf, name: &str) -> Option<String> {
        if let Some(str) = self.shaders.get(name) {
            Some(Self::shader(&str))
        } else {
            None
        }
    }

    pub fn get_shader_str_2tex(&self, _c: &GraphicsWindowConf, name: &str) -> Option<String> {
        if let Some(str) = self.shaders.get(name) {
            Some(Self::shader2tex(&str))
        } else {
            None
        }
    }

    pub fn get_shader_str_custom_prefix(
        &self,
        _c: &GraphicsWindowConf,
        name: &str,
        prefix: &str,
    ) -> Option<String> {
        if let Some(str) = self.shaders.get(name) {
            Some(Self::shader_custom_prefix(&str, prefix))
        } else {
            None
        }
    }

    pub fn get_graphics_ref(
        &self,
        c: &GraphicsWindowConf,
        name: &str,
    ) -> Option<GraphicsRefCustom<DefaultVertex>> {
        if let Some(str) = self.shaders.get(name) {
            Some(
                GraphicsCreator::<DefaultVertex>::default()
                    .with_mag_filter(wgpu::FilterMode::Nearest)
                    .to_graphics_ref(c, name, &Self::shader(&str)),
            )
        } else {
            None
        }
    }

    pub fn get_graphics_ref_2tex(
        &self,
        c: &GraphicsWindowConf,
        name: &str,
    ) -> Option<GraphicsRefCustom<DefaultVertex>> {
        if let Some(str) = self.shaders.get(name) {
            Some(
                GraphicsCreator::<DefaultVertex>::default()
                    .with_mag_filter(wgpu::FilterMode::Nearest)
                    .with_second_texture()
                    .to_graphics_ref(c, name, &Self::shader2tex(&str)),
            )
        } else {
            None
        }
    }

    pub fn has_changed(&self, other: &ControlShaderStrings) -> bool {
        self.shaders != other.shaders
    }

    pub fn naga_if_needed<VertexKind: GraphicsVertex>(
        &self,
        prev_shaders: &ControlShaderStrings,
    ) -> bool {
        if self.has_changed(&prev_shaders) {
            let mut all_success = true;

            for (name, shader_str) in self.shaders.iter() {
                let t = ShaderStrings::shader_str::<VertexKind>(&shader_str);
                if let Err(err) = naga::front::wgsl::parse_str(&t) {
                    println!(
                        "error with shader {:?}, {:?}, not updating until it works!",
                        name, err
                    );
                    all_success = false;
                }
            }

            all_success
        } else {
            false
        }
    }
}

impl ControlShaderStrings {
    fn to_normal(&self) -> ShaderStrings {
        ShaderStrings {
            shaders: self.shaders.clone(),
        }
    }

    pub fn should_update<VertexKind: GraphicsVertex>(
        &self,
        prev: &ControlShaderStrings,
        force_reload: bool,
    ) -> Option<ShaderStrings> {
        let shaders = self.to_normal();

        let shader_changed_and_compiles = shaders.naga_if_needed::<VertexKind>(&prev);

        if force_reload || shader_changed_and_compiles {
            // just in case there's lerp, be sure to use the one we tested
            Some(shaders)
        } else {
            None
        }
    }
}
