#![allow(dead_code)]
use glam::*;
use lerpable::Lerpable;
use murrelet_common::*;
use murrelet_draw::newtypes::*;
use murrelet_livecode_derive::Livecode;

use crate::{device_state::GraphicsWindowConf, graphics_ref::GraphicsRef};

pub trait ControlGraphics {
    fn update_graphics(&self, c: &GraphicsWindowConf, g: &GraphicsRef) {
        g.update_uniforms_other_tuple(c, self.more_info_other_tuple());
    }

    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]);
}

pub struct ControlGraphicsRef {
    pub label: &'static str,
    pub control: Box<dyn ControlGraphics>,
    graphics: GraphicsRef,
}
impl ControlGraphicsRef {
    pub fn new(
        label: &'static str,
        control: Box<dyn ControlGraphics>,
        graphics: Option<GraphicsRef>,
    ) -> Vec<ControlGraphicsRef> {
        // using a vec here to make it easier to concat with other lists
        if let Some(gg) = graphics {
            vec![ControlGraphicsRef {
                label,
                control,
                graphics: gg,
            }]
        } else {
            println!("missing ref! {:?}", label);
            // println!("missing ref!");
            vec![]
        }
    }
    pub fn update_graphics(&self, c: &GraphicsWindowConf) {
        self.control.update_graphics(c, &self.graphics);
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct GPUNoise {
    #[livecode(serde_default = "zeros")]
    #[lerpable(func = "lerpify_vec2")]
    offset: glam::Vec2,
    scale: f32,
    #[livecode(serde_default = "1")]
    alpha: f32,
    #[lerpable(func = "lerpify_vec2")]
    range: glam::Vec2,
}

impl GPUNoise {
    pub fn new(offset: Vec2, scale: f32, alpha: f32, range: Vec2) -> Self {
        Self {
            offset,
            scale,
            alpha,
            range,
        }
    }

    pub fn new_noise_texture(offset: Vec2, scale: f32, amount: f32) -> Self {
        Self {
            offset,
            scale,
            alpha: 1.0,
            range: vec2(-amount, amount),
        }
    }

    pub fn more_info(&self) -> ([f32; 4], [f32; 4]) {
        (
            [self.offset.x, self.offset.y, self.scale, self.alpha],
            [self.range.x, self.range.y, 0.0, 0.0],
        )
    }

    pub fn shader(c: &GraphicsWindowConf) -> GraphicsRef {
        prebuilt_shaders::new_shader_noise(c)
    }
}

impl ControlGraphics for GPUNoise {
    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]) {
        self.more_info()
    }
}

impl ControlGraphics for Vec2 {
    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]) {
        ([self.x, self.y, 0.0, 0.0], [0.0; 4])
    }
}

impl ControlGraphics for Vec3 {
    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]) {
        ([self.x, self.y, self.z, 0.0], [0.0; 4])
    }
}

impl ControlGraphics for MurreletColor {
    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]) {
        (self.into_rgba_components(), [0.0; 4])
    }
}

impl ControlGraphics for [f32; 4] {
    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]) {
        ([self[0], self[1], self[2], self[3]], [0.0; 4])
    }
}

impl ControlGraphics for f32 {
    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]) {
        ([*self, 0.0, 0.0, 0.0], [0.0; 4])
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct GPURGBAGradient {
    start: RGBandANewtype,
    end: RGBandANewtype,
}

impl GPURGBAGradient {
    pub fn more_info(&self) -> ([f32; 4], [f32; 4]) {
        (self.start.rgba(), self.end.rgba())
    }
}

impl ControlGraphics for GPURGBAGradient {
    fn more_info_other_tuple(&self) -> ([f32; 4], [f32; 4]) {
        self.more_info()
    }
}

pub mod prebuilt_shaders {

    use crate::{
        device_state::GraphicsWindowConf,
        gpu_macros::ShaderStr,
        graphics_ref::{GraphicsCreator, GraphicsRef},
        *,
    };

    pub fn new_shader_basic(c: &GraphicsWindowConf, name: &str, shader: &str) -> GraphicsRef {
        GraphicsCreator::default().to_graphics_ref(c, name, shader)
    }

    pub fn new_shader_2tex(c: &GraphicsWindowConf, name: &str, shader: &str) -> GraphicsRef {
        let name = format!("{} {:?}", name, c.dims);
        GraphicsCreator::default()
            .with_second_texture()
            .to_graphics_ref(c, &name, shader)
    }

    /// fbm noise. use with ControlNoise
    /// # Attributes
    /// ## Textures
    /// N/A
    /// ## Uniforms
    /// - 0.xy: offset for noise
    /// - 0.z: scale, default to 1
    /// - 0.a: default alpha, if 0 will set to noise val instead
    /// - 1.x: min value for noise
    /// - 1.y: max value for noise
    /// ## Returns
    /// - GraphicsRef
    pub fn new_shader_noise(c: &GraphicsWindowConf) -> GraphicsRef {
        let shader: String = build_shader_2tex! {
            (
                raw r###"

                let offset: vec2<f32> = uniforms.more_info.xy;
                let scale: f32 = uniforms.more_info.z;
                let raw_alpha: f32 = uniforms.more_info.a;
                let min_val: f32 = uniforms.more_info_other.x;
                let max_val: f32 = uniforms.more_info_other.y;

                let noise_coords: vec2<f32> = scale * (tex_coords + offset);

                let n: f32 = fbm(noise_coords);

                let alpha = max(raw_alpha, n);

                let noise_val = min_val + (max_val - min_val) * n;

                let result = vec4<f32>(vec3<f32>(noise_val), alpha);
                "###;
            )
        };
        let g = new_shader_2tex(c, "fbm noise", &shader);
        g.update_uniforms_other(c, [0.0, 0.0, 1.0, 1.0], [0.0, 1.0, 0.0, 0.0]);
        g
    }
}
