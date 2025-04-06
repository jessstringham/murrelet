// defines the SVG basic shapes

use glam::{Mat4, Vec2};
use lerpable::Lerpable;
use murrelet_gui::MurreletGUI;
use murrelet_livecode_derive::Livecode;

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub struct SvgRect {
    #[livecode(serde_default = "0")]
    pub x: f32,
    #[livecode(serde_default = "0")]
    pub y: f32,
    #[livecode(serde_default = "0")]
    pub rx: f32, // x corner radius
    #[livecode(serde_default = "0")]
    pub ry: f32, // y corner radius
    pub width: f32,
    pub height: f32,
}

impl SvgRect {
    pub fn new_centered(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rx: 0.0,
            ry: 0.0,
            width,
            height,
        }
    }

    pub fn new_at_loc(loc: Vec2, w_h: Vec2) -> Self {
        Self {
            x: loc.x,
            y: loc.y,
            rx: 0.0,
            ry: 0.0,
            width: w_h.x,
            height: w_h.y,
        }
    }
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub struct SvgCircle {
    #[livecode(serde_default = "0")]
    pub x: f32,
    #[livecode(serde_default = "0")]
    pub y: f32,
    #[livecode(serde_default = "1")]
    pub r: f32,
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub enum SvgShape {
    Rect(SvgRect),
    Circle(SvgCircle),
}

#[derive(Clone, Debug)]
pub struct TransformedSvgShape {
    pub shape: SvgShape,
    pub t: Mat4,
}
impl TransformedSvgShape {
    pub fn from_shape(shape: SvgShape) -> Self {
        Self {
            shape,
            t: Mat4::IDENTITY,
        }
    }

    pub fn transform_with_mat4_after(&self, t: Mat4) -> Self {
        Self {
            shape: self.shape.clone(),
            t: t * self.t,
        }
    }

    pub fn transform_with_mat4_before(&self, t: Mat4) -> Self {
        Self {
            shape: self.shape.clone(),
            t: self.t * t,
        }
    }
}
