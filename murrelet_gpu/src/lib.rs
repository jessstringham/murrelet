pub mod compute;
pub mod device_state;
pub mod editable_shaders;
pub mod gpu_livecode;
pub mod gpu_macros;
pub mod graphics_ref;
pub mod headless;
pub mod headless_harness;
pub mod headless_macros;
pub mod shader_str;
pub mod uniforms;
pub mod window;

#[cfg(feature = "nannou")]
pub use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
pub use wgpu_for_latest as wgpu;

pub use graphics_ref::MurreletTexture;
pub use headless_harness::{HeadlessHarness, HeadlessJob};
pub use murrelet_draw::drawable::ToMixedDrawables;
pub use murrelet_gpu_derive::*;

use std::collections::HashMap;

use murrelet_common::StrId;

#[derive(Clone, Debug)]
pub struct TextureMap(HashMap<StrId, MurreletTexture>);
impl TextureMap {
    pub fn combine(&mut self, texture_map: TextureMap) {
        self.0.extend(texture_map.0.into_iter())
    }

    pub fn new(v: Vec<(StrId, MurreletTexture)>) -> Self {
        Self(v.into_iter().collect())
    }

    pub fn get(&self, src: &StrId) -> Option<&MurreletTexture> {
        self.0.get(src)
    }

    pub fn empty() -> TextureMap {
        Self::new(vec![])
    }
}
