// defines the SVG basic shapes

use glam::Mat4;
use lerpable::Lerpable;
use murrelet_livecode_derive::Livecode;


#[derive(Clone, Debug, Livecode, Lerpable)]
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




#[derive(Clone, Debug, Livecode, Lerpable)]
pub enum SvgShape {
    Rect(SvgRect)
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
