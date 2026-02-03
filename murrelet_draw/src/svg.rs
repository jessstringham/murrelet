// defines the SVG basic shapes

use glam::Vec2;
use lerpable::Lerpable;
use lyon::geom::{euclid::Point2D, Point};
use murrelet_common::{SimpleTransform2d, ToSimpleTransform};
use murrelet_gui::MurreletGUI;
use murrelet_livecode_derive::Livecode;

use crate::curve_drawer::{CurveArc, CurveDrawer};

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

    pub fn to(&self) -> Vec2 {
        (self.x, self.y).into()
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
    pub fn to(&self) -> Vec2 {
        (self.to_x, self.to_y).into()
    }

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
pub struct SvgArc {
    radius_x: f32,
    radius_y: f32,
    x_axis_rotation: f32,
    large_arc_flag: f32,
    sweep_flag: f32,
    to_x: f32,
    to_y: f32,
}

impl SvgArc {
    pub fn to(&self) -> Vec2 {
        (self.to_x, self.to_y).into()
    }

    pub fn new(
        radius_x: f32,
        radius_y: f32,
        x_axis_rotation: f32,
        large_arc_flag: f32,
        sweep_flag: f32,
        to_x: f32,
        to_y: f32,
    ) -> Self {
        Self {
            radius_x,
            radius_y,
            x_axis_rotation,
            large_arc_flag,
            sweep_flag,
            to_x,
            to_y,
        }
    }

    pub fn from_arc(a: &CurveArc) -> Self {
        let [a, b, c, d, e, f, g] = a.svg_params();
        SvgArc::new(a, b, c, d, e, f, g)
    }

    pub fn params(&self) -> (f32, f32, f32, f32, f32, f32, f32) {
        (
            self.radius_x,
            self.radius_y,
            self.x_axis_rotation,
            self.large_arc_flag,
            self.sweep_flag,
            self.to_x,
            self.to_y,
        )
    }
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub enum SvgCmd {
    Line(SvgTo),
    CubicBezier(SvgCubicBezier),
    ArcTo(SvgArc),
}
impl SvgCmd {
    pub fn to(&self) -> Vec2 {
        match self {
            SvgCmd::Line(svg_to) => svg_to.to(),
            SvgCmd::CubicBezier(svg_cubic_bezier) => svg_cubic_bezier.to(),
            SvgCmd::ArcTo(svg_arc) => svg_arc.to(),
        }
    }
}

#[derive(Clone, Debug, Livecode, MurreletGUI, Lerpable)]
pub struct SvgPathDef {
    start: SvgTo,
    v: Vec<SvgCmd>,
}

impl SvgPathDef {
    pub fn start(&self) -> Vec2 {
        (self.start.x, self.start.y).into()
    }

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

    fn add_curve_points(&mut self, curve_points: &crate::curve_drawer::CurvePoints) {
        for c in curve_points.points() {
            self.add_line(*c);
        }
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

    fn add_curve_bezier(&mut self, curve_cubic_bezier: &crate::curve_drawer::CurveCubicBezier) {
        self.add_cubic_bezier(
            curve_cubic_bezier.ctrl1(),
            curve_cubic_bezier.ctrl2(),
            curve_cubic_bezier.to(),
        );
    }

    fn add_curve_arc(&mut self, curve_arc: &CurveArc) {
        if let Some((hemi1, hemi2)) = curve_arc.is_full_circle_then_split() {
            self.v.push(SvgCmd::ArcTo(SvgArc::from_arc(&hemi1)));
            self.v.push(SvgCmd::ArcTo(SvgArc::from_arc(&hemi2)));
        } else {
            self.v.push(SvgCmd::ArcTo(SvgArc::from_arc(curve_arc)));
        }
    }

    // fn add_curve_arc(&mut self, curve_arc: &CurveArc) {
    //     self.v.push(SvgCmd::ArcTo(SvgArc::from_arc(curve_arc)));
    // }

    pub fn svg_move_to(&self) -> (f32, f32) {
        (self.start.x, self.start.y)
    }

    pub fn cmds(&self) -> &[SvgCmd] {
        &self.v
    }

    fn from_cd(cd: &CurveDrawer) -> SvgPathDef {
        // if it's a weird cd, then just draw a dot at 0, 0 and be done...
        // probably want to do better error handling here
        let mut curr = cd.first_point().unwrap_or_default();
        let mut path = SvgPathDef::new(curr);

        for s in cd.segments() {
            if curr != s.first_point() {
                path.add_line(s.first_point());
            }

            match s {
                crate::curve_drawer::CurveSegment::Arc(curve_arc) => {
                    path.add_curve_arc(curve_arc);
                }
                crate::curve_drawer::CurveSegment::Points(curve_points) => {
                    path.add_curve_points(curve_points);
                }
                crate::curve_drawer::CurveSegment::CubicBezier(curve_cubic_bezier) => {
                    path.add_curve_bezier(curve_cubic_bezier);
                }
            }

            curr = s.last_point();
        }

        path
    }
}

pub fn glam_to_lyon(vec: Vec2) -> Point2D<f32, lyon::geom::euclid::UnknownUnit> {
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

    pub fn transform<F: ToSimpleTransform>(&self, t: &F) -> TransformedSvgShape {
        TransformedSvgShape {
            shape: self.clone(),
            t: t.to_simple_transform(),
        }
    }

    pub fn as_transform(&self) -> TransformedSvgShape {
        TransformedSvgShape {
            shape: self.clone(),
            t: SimpleTransform2d::noop(),
        }
    }

    pub(crate) fn as_path(&self) -> SvgPathDef {
        match self {
            SvgShape::Rect(_) => todo!(),
            SvgShape::Circle(_) => todo!(),
            SvgShape::Path(svg_path_def) => svg_path_def.clone(),
        }
    }

    pub(crate) fn circle(loc: Vec2, rad: f32) -> SvgShape {
        SvgShape::Circle(SvgCircle {
            x: loc.x,
            y: loc.y,
            r: rad,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TransformedSvgShape {
    pub shape: SvgShape,
    pub t: SimpleTransform2d,
}
impl TransformedSvgShape {
    pub fn from_shape(shape: SvgShape) -> Self {
        Self {
            shape,
            t: SimpleTransform2d::noop(),
        }
    }

    pub fn from_cd(cd: &CurveDrawer) -> Self {
        let shape = SvgShape::Path(SvgPathDef::from_cd(&cd));

        Self {
            shape,
            t: SimpleTransform2d::noop(),
        }
    }

    pub fn transform_after<F: ToSimpleTransform>(&self, t: &F) -> Self {
        Self {
            shape: self.shape.clone(),
            t: self.t.add_transform_after(t),
        }
    }

    pub fn transform_before<F: ToSimpleTransform>(&self, t: &F) -> Self {
        Self {
            shape: self.shape.clone(),
            t: self.t.add_transform_before(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::vec2;
    use murrelet_common::AnglePi;

    use super::*;

    #[test]
    fn test_curve_drawer_to_svg_to_curve_drawer_arc() {
        let cd = CurveDrawer::new_simple_arc(
            Vec2::new(20.0, 20.0),
            5.0,
            AnglePi::new(-1.0),
            AnglePi::new(0.0),
        );

        let first_point_before = cd.first_point().unwrap();
        let last_point_before = cd.last_point().unwrap();

        assert_eq!(first_point_before, vec2(15.0, 20.0));
        assert_eq!(last_point_before, vec2(25.0, 20.0));

        // Convert to SvgPathDef
        let svg_path = SvgPathDef::from_cd(&cd);

        assert_eq!(svg_path.start(), first_point_before);
        let cmds = svg_path.cmds().to_vec();
        assert_eq!(cmds.len(), 1);

        assert_eq!(cmds[0].to(), last_point_before);
    }
}
