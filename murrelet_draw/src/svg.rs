// defines the SVG basic shapes

use glam::{Mat4, Vec2};
use lerpable::Lerpable;
use lyon::geom::{euclid::Point2D, Point};
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
            x: -width / 2.0,
            y: -height / 2.0,
            rx: 0.0,
            ry: 0.0,
            width,
            height,
        }
    }

    pub fn new_at_loc(loc: Vec2, w_h: Vec2) -> Self {
        Self {
            x: loc.x - w_h.x / 2.0,
            y: loc.y - w_h.y / 2.0,
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
pub struct SvgTo {
    x: f32,
    y: f32,
}
impl SvgTo {
    pub fn params(&self) -> (f32, f32) {
        (self.x, self.y)
    }
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub struct SvgCubicBezier {
    ctrl1_x: f32,
    ctrl1_y: f32,
    ctrl2_x: f32,
    ctrl2_y: f32,
    to_x: f32,
    to_y: f32,
}
impl SvgCubicBezier {
    pub fn params(&self) -> (f32, f32, f32, f32, f32, f32) {
        (
            self.ctrl1_x,
            self.ctrl1_y,
            self.ctrl2_x,
            self.ctrl2_y,
            self.to_x,
            self.to_y,
        )
    }
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub enum SvgCmd {
    Line(SvgTo),
    CubicBezier(SvgCubicBezier),
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub struct SvgPathDef {
    start: SvgTo,
    v: Vec<SvgCmd>,
}

impl SvgPathDef {
    pub fn new(start: Vec2) -> Self {
        Self {
            start: SvgTo {
                x: start.x,
                y: start.y,
            },
            v: Vec::new(),
        }
    }

    pub fn add_line(&mut self, to: Vec2) {
        self.v.push(SvgCmd::Line(SvgTo { x: to.x, y: to.y }));
    }

    pub fn add_cubic_bezier(&mut self, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) {
        self.v.push(SvgCmd::CubicBezier(SvgCubicBezier {
            ctrl1_x: ctrl1.x,
            ctrl1_y: ctrl1.y,
            ctrl2_x: ctrl2.x,
            ctrl2_y: ctrl2.y,
            to_x: to.x,
            to_y: to.y,
        }))
    }

    pub fn svg_move_to(&self) -> (f32, f32) {
        (self.start.x, self.start.y)
    }

    pub fn cmds(&self) -> &[SvgCmd] {
        &self.v
    }
}

fn glam_to_lyon(vec: Vec2) -> Point2D<f32, lyon::geom::euclid::UnknownUnit> {
    Point::new(vec.x, vec.y)
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub enum SvgShape {
    Rect(SvgRect),
    Circle(SvgCircle),
    Path(SvgPathDef),
}

impl SvgShape {
    pub fn new_rect_at_loc(loc: Vec2, w_h: Vec2) -> Self {
        SvgShape::Rect(SvgRect::new_at_loc(loc, w_h))
    }

    pub fn new_centered_rect(width: f32, height: f32) -> Self {
        SvgShape::Rect(SvgRect::new_centered(width, height))
    }
    pub fn new_centered_circle(radius: f32) -> Self {
        SvgShape::Circle(SvgCircle {
            x: 0.0,
            y: 0.0,
            r: radius,
        })
    }

    pub fn transform(&self, t: Mat4) -> TransformedSvgShape {
        TransformedSvgShape {
            shape: self.clone(),
            t,
        }
    }

    pub fn as_transform(&self) -> TransformedSvgShape {
        TransformedSvgShape {
            shape: self.clone(),
            t: Mat4::IDENTITY,
        }
    }

    pub(crate) fn as_path(&self) -> SvgPathDef {
        match self {
            SvgShape::Rect(_) => todo!(),
            SvgShape::Circle(_) => todo!(),
            SvgShape::Path(svg_path_def) => svg_path_def.clone(),
        }
    }
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
