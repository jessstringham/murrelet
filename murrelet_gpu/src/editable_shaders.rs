use std::collections::HashMap;

use crate::{gpu_macros::ShaderStr, window::GraphicsWindowConf};
use lerpable::Lerpable;
use murrelet_livecode_derive::Livecode;
use wgpu_for_latest::naga;
#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

use crate::{
    build_shader, build_shader_2tex,
    graphics_ref::{GraphicsCreator, GraphicsRef},
};

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct ShaderStrings {
    #[livecode(kind = "none")]
    #[lerpable(method = "skip")]
    shaders: HashMap<String, String>,
}
impl ShaderStrings {
    fn shader(shader: &str) -> String {
        build_shader! {
            (
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

    pub fn get_graphics_ref(&self, c: &GraphicsWindowConf, name: &str) -> Option<GraphicsRef> {
        if let Some(str) = self.shaders.get(name) {
            Some(
                GraphicsCreator::default()
                    .with_mag_filter(wgpu::FilterMode::Nearest)
                    .to_graphics_ref(c, name, &Self::shader(&str)),
            )
        } else {
            None
        }
    }

    pub fn get_graphics_ref_2tex(&self, c: &GraphicsWindowConf, name: &str) -> Option<GraphicsRef> {
        if let Some(str) = self.shaders.get(name) {
            Some(
                GraphicsCreator::default()
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

    pub fn naga_if_needed(&self, prev_shaders: &ControlShaderStrings) -> bool {
        if self.has_changed(&prev_shaders) {
            let mut all_success = true;

            for (name, shader_str) in self.shaders.iter() {
                let t = ShaderStrings::shader2tex(&shader_str);
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

    pub fn should_update(
        &self,
        prev: &ControlShaderStrings,
        force_reload: bool,
    ) -> Option<ShaderStrings> {
        let shaders = self.to_normal();

        let shader_changed_and_compiles = shaders.naga_if_needed(&prev);

        if force_reload || shader_changed_and_compiles {
            // just in case there's lerp, be sure to use the one we tested
            Some(shaders)
        } else {
            None
        }
    }
}
