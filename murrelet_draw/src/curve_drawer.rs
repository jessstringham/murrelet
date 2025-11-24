use glam::*;
use itertools::Itertools;
use lerpable::Lerpable;
use lyon::{geom::CubicBezierSegment, path::Path};
use murrelet_common::*;
use murrelet_livecode_derive::*;
use serde::{Deserialize, Serialize};
use svg::node::element::path::Data;

use crate::{
    cubic::CubicBezier,
    newtypes::*,
    svg::glam_to_lyon,
    tesselate::{
        cubic_bezier_path_to_lyon, flatten_cubic_bezier_path,
        flatten_cubic_bezier_path_with_tolerance, parse_svg_data_as_vec2, ToVecVec2,
    },
};

#[derive(Debug, Clone, Default, Livecode, Lerpable, Serialize, Deserialize)]
pub enum CubicOptionVec2 {
    #[default]
    None,
    Some(Vec2Newtype),
}
impl CubicOptionVec2 {
    pub fn or_last(&self, anchor: Vec2, last_ctrl2: Vec2) -> Vec2 {
        match self {
            CubicOptionVec2::None => anchor * 2.0 - last_ctrl2,
            CubicOptionVec2::Some(vec2_newtype) => vec2_newtype.vec2(),
        }
    }

    pub fn none() -> Self {
        Self::None
    }

    pub fn some(v: Vec2) -> Self {
        Self::Some(Vec2Newtype::new(v))
    }
}

#[derive(Debug, Clone, Default, Livecode, Lerpable, Serialize, Deserialize)]
pub struct CubicBezierTo {
    pub ctrl1: CubicOptionVec2,

    // #[serde(serialize_with = "serialize_vec2")]
    pub ctrl2: Vec2,
    // #[serde(serialize_with = "serialize_vec2")]
    pub to: Vec2,
}

impl CubicBezierTo {
    pub fn new(ctrl1o: Option<Vec2>, ctrl2: Vec2, to: Vec2) -> Self {
        let ctrl1 = match ctrl1o {
            Some(c) => CubicOptionVec2::some(c),
            None => CubicOptionVec2::none(),
        };
        Self { ctrl1, ctrl2, to }
    }
}

#[derive(Debug, Clone, Default, Livecode, Lerpable, Serialize, Deserialize)]
pub struct CubicBezierPath {
    pub from: Vec2,
    pub ctrl1: Vec2,
    pub curves: Vec<CubicBezierTo>,
    pub closed: bool,
}
impl CubicBezierPath {
    pub fn new(from: Vec2, ctrl1: Vec2, curves: Vec<CubicBezierTo>, closed: bool) -> Self {
        Self {
            from,
            ctrl1,
            curves,
            closed,
        }
    }

    pub fn to_vec2_count(&self, count: usize) -> Vec<Vec2> {
        let len = self.to_cd().length();
        let line_space = len / count as f32;

        let svg = self.to_data();
        let path = parse_svg_data_as_vec2(&svg, line_space);

        // if let Some(a) = path.last() {
        //     if a.distance(self.to.yx()) > 1.0e-3 {
        //         path.push(self.to.yx())
        //     }
        // }

        path.into_iter().map(|x| vec2(x.y, x.x)).collect_vec()
    }

    pub fn to_cd(&self) -> CurveDrawer {
        let mut cs = vec![];
        for c in self.to_cubic() {
            cs.push(CurveSegment::cubic(c));
        }
        CurveDrawer::new(cs, self.closed)
    }

    pub fn to_cubic(&self) -> Vec<CubicBezier> {
        let mut svg = vec![];

        let mut from = self.from;

        let mut last_ctrl1 = self.ctrl1;

        let mut first_ctrl1_used: Option<Vec2> = None;

        for s in &self.curves {
            let ctrl1 = s.ctrl1.or_last(from, last_ctrl1);
            if first_ctrl1_used.is_none() {
                first_ctrl1_used = Some(ctrl1);
            }
            svg.push(CubicBezier::new(from, ctrl1, s.ctrl2, s.to));
            last_ctrl1 = s.ctrl2;
            from = s.to;
        }

        if self.closed {
            let ctrl1 = CubicOptionVec2::none().or_last(from, last_ctrl1);
            // let ctrl2 = CubicOptionVec2::none().or_last(self.from, self.ctrl1);
            let ctrl2_source = first_ctrl1_used.unwrap_or(self.ctrl1);
            let ctrl2 = CubicOptionVec2::none().or_last(self.from, ctrl2_source);
            svg.push(CubicBezier::new(from, ctrl1, ctrl2, self.from));
        }

        svg
    }

    pub fn to_data(&self) -> Data {
        let mut svg = svg::node::element::path::Data::new();

        let x = self.from.x;
        let y = self.from.y;
        let start: svg::node::element::path::Parameters = vec![x, y].into();
        svg = svg.move_to(start);

        let mut last = self.from;
        let mut last_ctrl1 = self.ctrl1;

        for s in &self.curves {
            let ctrl1 = s.ctrl1.or_last(last, last_ctrl1);

            let cubic: svg::node::element::path::Parameters =
                vec![ctrl1.x, ctrl1.y, s.ctrl2.x, s.ctrl2.y, s.to.x, s.to.y].into();
            svg = svg.cubic_curve_to(cubic);

            last_ctrl1 = s.ctrl2;
            last = s.to;
        }
        svg
    }

    pub fn to_lyon(&self) -> Option<Path> {
        cubic_bezier_path_to_lyon(&self.to_cubic(), self.closed)
    }

    pub fn to_vec2(&self) -> Option<Vec<Vec2>> {
        flatten_cubic_bezier_path(&self.to_cubic(), self.closed)
    }

    pub fn last_point(&self) -> Vec2 {
        if let Some(last) = self.curves.last() {
            last.to
        } else {
            self.from
        }
    }

    pub fn first_point(&self) -> Vec2 {
        self.from
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct CurveDrawer {
    pub segments: Vec<CurveSegment>,
    pub closed: bool, // this is mostly used for algorithms that use curve drawers. you'll need to use a style that's closed
}

impl CurveDrawer {
    pub fn new(segments: Vec<CurveSegment>, closed: bool) -> Self {
        Self { segments, closed }
    }

    pub fn new_closed(segments: Vec<CurveSegment>) -> Self {
        Self {
            segments,
            closed: true,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn segments(&self) -> &[CurveSegment] {
        &self.segments
    }

    pub fn add_segment(&mut self, cs: CurveSegment) {
        self.segments.push(cs);
    }

    pub fn new_simple_arc<A: IsAngle>(loc: Vec2, radius: f32, start: A, end: A) -> Self {
        CurveDrawer::new(
            vec![CurveSegment::new_simple_arc(loc, radius, start, end)],
            false,
        )
    }

    pub fn new_simple_sector<A: IsAngle>(loc: Vec2, radius: f32, start: A, end: A) -> Self {
        CurveDrawer::new(
            vec![
                CurveSegment::new_simple_point(loc),
                CurveSegment::Arc(CurveArc::new(loc, radius, start, end)),
            ],
            true,
        )
    }

    pub fn new_simple_circle(loc: Vec2, radius: f32) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_circle(loc, radius)], true)
    }

    pub fn new_from_circle(c: &Circle) -> Self {
        CurveDrawer::new_simple_circle(c.center, c.radius)
    }

    pub fn new_simple_line(start: Vec2, end: Vec2) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_line(start, end)], false)
    }

    pub fn new_simple_points(vs: Vec<Vec2>, closed: bool) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_points(vs)], closed)
    }

    pub fn new_simple_polyline(vs: Polyline, closed: bool) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_points(vs.into_vec())], closed)
    }

    pub fn as_closed(&self) -> Self {
        let mut new = self.clone();
        new.closed = true;
        new
    }

    pub fn noop() -> Self {
        Self::new(vec![], false)
    }

    pub fn first_point(&self) -> Option<Vec2> {
        let first_command = self.segments().first()?;
        Some(first_command.first_point())
    }

    pub fn last_point(&self) -> Option<Vec2> {
        let last_command = self.segments().last()?;
        Some(last_command.last_point())
    }

    pub fn length(&self) -> f32 {
        self.segments
            .iter()
            .map(|segment| match segment {
                CurveSegment::CubicBezier(c) => {
                    let lyon_cubic = CubicBezierSegment {
                        from: glam_to_lyon(c.from),
                        ctrl1: glam_to_lyon(c.ctrl1),
                        ctrl2: glam_to_lyon(c.ctrl2),
                        to: glam_to_lyon(c.to),
                    };
                    lyon_cubic.approximate_length(0.1)
                }
                CurveSegment::Points(p) => p
                    .points()
                    .windows(2)
                    .map(|pts| pts[0].distance(pts[1]))
                    .sum(),
                CurveSegment::Arc(a) => {
                    let angle_diff = (a.end_pi.angle_pi() - a.start_pi.angle_pi()).rem_euclid(2.0);
                    let sweep_angle_rads = angle_diff * std::f32::consts::PI;
                    a.radius * sweep_angle_rads
                }
            })
            .sum()
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub enum CurveSegment {
    Arc(CurveArc),
    Points(CurvePoints),
    CubicBezier(CurveCubicBezier),
}

impl CurveSegment {
    pub fn arc<A: IsAngle>(loc: Vec2, radius: f32, start_pi: A, end_pi: A) -> Self {
        Self::Arc(CurveArc {
            loc,
            radius,
            start_pi: start_pi.as_angle_pi(),
            end_pi: end_pi.as_angle_pi(),
        })
    }

    pub fn first_point(&self) -> Vec2 {
        match self {
            CurveSegment::Arc(c) => c.first_point(),
            CurveSegment::Points(c) => c.first_point(),
            CurveSegment::CubicBezier(c) => c.first_point(),
        }
    }

    pub fn last_point(&self) -> Vec2 {
        match self {
            CurveSegment::Arc(c) => c.last_point(),
            CurveSegment::Points(c) => c.last_point(),
            CurveSegment::CubicBezier(c) => c.last_point(),
        }
    }

    pub fn reverse(&self) -> Self {
        match self {
            CurveSegment::Arc(curve_arc) => CurveSegment::Arc(curve_arc.reverse()),
            CurveSegment::Points(curve_points) => CurveSegment::Points(curve_points.reverse()),
            CurveSegment::CubicBezier(c) => {
                // CurveSegment::CubicBezier(c.reverse())
                CurveSegment::Points(c.as_points().reverse())
            }
        }
    }

    pub fn new_simple_arc<Rad: IsLength, A1: IsAngle, A2: IsAngle>(
        loc: Vec2,
        radius: Rad,
        start: A1,
        end: A2,
    ) -> Self {
        CurveSegment::Arc(CurveArc::new(loc, radius.len(), start, end))
    }

    // pub fn new_simple_arc_from<Rad: IsLength, A1: IsAngle, A2: IsAngle>(
    //     start: Vec2,
    //     radius: Rad,
    //     in_angle: A1,
    //     angle_length: A2,
    //     ccw: bool,
    // ) -> Self {
    //     // calculate the center
    //     let (loc, end_pi) = if ccw {
    //         (
    //             start + in_angle.to_norm_dir() * radius.len(),
    //             in_angle.as_angle() - angle_length.as_angle(),
    //         )
    //     } else {
    //         (
    //             start - in_angle.to_norm_dir() * radius.len(),
    //             in_angle.as_angle() + angle_length.as_angle(),
    //         )
    //     };

    //     CurveSegment::Arc(CurveArc::new(loc, radius.len(), in_angle, end_pi))
    // }

    pub fn new_simple_circle(loc: Vec2, radius: f32) -> Self {
        CurveSegment::Arc(CurveArc::new(
            loc,
            radius,
            AnglePi::new(0.0),
            AnglePi::new(2.0),
        ))
    }

    pub fn new_simple_point(point: Vec2) -> Self {
        CurveSegment::new_simple_points(vec![point])
    }

    pub fn new_simple_line(start: Vec2, end: Vec2) -> Self {
        CurveSegment::new_simple_points(vec![start, end])
    }

    pub fn new_simple_points(points: Vec<Vec2>) -> Self {
        CurveSegment::Points(CurvePoints::new(points))
    }

    pub fn pt_count(&self) -> Option<usize> {
        match self {
            CurveSegment::Arc(_) => None,
            CurveSegment::Points(p) => Some(p.points.len()),
            CurveSegment::CubicBezier(_) => None,
        }
    }

    pub fn first_angle(&self) -> Option<AnglePi> {
        Some(self.first_spot().angle().into()) // todo, return none if it's one point
    }

    pub fn first_spot(&self) -> SpotOnCurve {
        match self {
            CurveSegment::Arc(arc) => {
                let a = CurveArc::new(arc.loc, arc.radius, arc.start_pi, arc.end_pi);
                SpotOnCurve::new(a.first_point(), a.start_tangent_angle())
            }
            CurveSegment::Points(points) => {
                let vec2s = points.points();
                if vec2s.len() >= 2 {
                    let first = vec2s[0];
                    let second = vec2s[1];

                    let angle = PointToPoint::new(first, second).angle().perp_to_left();
                    SpotOnCurve::new(first, angle)
                } else {
                    todo!()
                }
            }
            CurveSegment::CubicBezier(cubic_bezier) => cubic_bezier.to_cubic().start_to_tangent().0,
        }
    }

    pub fn last_spot(&self) -> SpotOnCurve {
        match self {
            CurveSegment::Arc(arc) => {
                let a = CurveArc::new(arc.loc, arc.radius, arc.start_pi, arc.end_pi);
                SpotOnCurve::new(a.last_point(), a.end_tangent_angle())
            }
            CurveSegment::Points(p) => {
                if p.points().len() >= 2 {
                    let points = p.points();
                    let end = *points.last().unwrap();
                    let prev = *points.get(points.len() - 2).unwrap();
                    let angle = PointToPoint::new(prev, end).angle().perp_to_left();
                    SpotOnCurve::new(end, angle)
                } else {
                    unimplemented!() // need to look at the val before...
                }
            }
            CurveSegment::CubicBezier(c) => c.to_cubic().end_to_tangent().0,
        }
    }

    pub fn cubic_bezier(from: Vec2, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> Self {
        Self::CubicBezier(CurveCubicBezier {
            from,
            ctrl1,
            ctrl2,
            to,
        })
    }

    fn cubic(c: CubicBezier) -> CurveSegment {
        Self::CubicBezier(CurveCubicBezier::from_cubic(c))
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct CurveArc {
    #[livecode(serde_default = "zeros")]
    pub loc: Vec2, // center of circle
    pub radius: f32,
    pub start_pi: AnglePi,
    pub end_pi: AnglePi,
}
impl CurveArc {
    pub fn new<A1: IsAngle, A2: IsAngle>(loc: Vec2, radius: f32, start_pi: A1, end_pi: A2) -> Self {
        Self {
            loc,
            radius,
            start_pi: start_pi.as_angle_pi(),
            end_pi: end_pi.as_angle_pi(),
        }
    }

    pub fn is_in_arc(&self, angle: AnglePi) -> bool {
        let start_pi = self.start_pi.angle_pi();
        let end_pi = self.end_pi.angle_pi();

        if (end_pi - start_pi).abs() >= 2.0 {
            true
        } else {
            // eh, try to align it
            let angle_pi = angle.angle_pi();

            if self.is_ccw() {
                angle_pi <= end_pi && angle_pi >= start_pi
            } else {
                angle_pi <= start_pi && angle_pi >= end_pi
            }
        }
    }

    pub fn is_ccw(&self) -> bool {
        self.end_pi.angle_pi() > self.start_pi.angle_pi()
    }

    pub fn reverse(&self) -> Self {
        Self {
            start_pi: self.end_pi,
            end_pi: self.start_pi,
            ..*self
        }
    }

    // useful for svg
    pub fn is_large_arc(&self) -> bool {
        // (self.end_pi.angle_pi() - self.start_pi.angle_pi()).abs() > 1.0
        // (self.end_pi.angle_pi() - self.start_pi.angle_pi()).rem_euclid(2.0) > 1.0

        let ccw_angular_distance =
            (self.end_pi.angle_pi() - self.start_pi.angle_pi()).rem_euclid(2.0);

        if self.is_ccw() {
            ccw_angular_distance > 1.0
        } else {
            // for CW, the distance is the other way around the circle
            (2.0 - ccw_angular_distance) > 1.0
        }
    }

    pub fn last_point(&self) -> Vec2 {
        let curr_angle = self.end_pi.angle();
        let (loc_sin, loc_cos) = curr_angle.sin_cos();
        vec2(loc_cos, loc_sin) * self.radius + self.loc
    }

    pub fn first_point(&self) -> Vec2 {
        let curr_angle = self.start_pi.angle();
        let (loc_sin, loc_cos) = curr_angle.sin_cos();
        vec2(loc_cos, loc_sin) * self.radius + self.loc
    }

    // angle tangent to the end point
    pub fn end_tangent_angle(&self) -> Angle {
        if self.is_ccw() {
            self.end_angle().perp_to_left()
        } else {
            self.end_angle().perp_to_right()
        }
    }

    fn end_angle(&self) -> Angle {
        self.end_pi.as_angle()
        // AnglePi::new(self.end_pi).into()
    }

    fn start_angle(&self) -> Angle {
        self.start_pi.as_angle()
        // AnglePi::new(self.start_pi).into()
    }

    pub fn start_tangent_angle(&self) -> Angle {
        if self.is_ccw() {
            self.start_angle().perp_to_left()
        } else {
            self.start_angle().perp_to_right()
        }
    }

    pub fn set_radius(&self, radius: f32) -> CurveArc {
        let mut m = self.clone();
        m.radius = radius;
        m
    }

    pub fn svg_params(&self) -> [f32; 7] {
        // assumes you've already moved to first_point
        let last_point = self.last_point();
        [
            self.radius,
            self.radius, // same as other rad because it's a circle
            0.0,         // angle of ellipse doesn't matter, so 0
            if self.is_large_arc() { 1.0 } else { 0.0 }, // large arc flag
            if self.is_ccw() { 1.0 } else { 0.0 }, // sweep-flag
            last_point.x,
            last_point.y,
        ]
    }

    pub fn is_full_circle(&self) -> bool {
        (self.end_pi - self.start_pi).angle_pi().abs() >= 0.0
            && (self.end_pi - self.start_pi).angle_pi().rem_euclid(2.0) < 1e-3f32
    }

    pub fn is_full_circle_then_split(&self) -> Option<(CurveArc, CurveArc)> {
        if self.is_full_circle() {
            let start_angle = self.start_angle();
            let mid_angle = self.start_angle() + (self.end_angle() - self.start_angle()).scale(0.5);
            let end_angle = self.end_angle();

            let semi_circle1 = CurveArc::new(self.loc, self.radius, start_angle, mid_angle);
            let semi_circle2 = CurveArc::new(self.loc, self.radius, mid_angle, end_angle);
            Some((semi_circle1, semi_circle2))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct CurveCubicBezier {
    from: Vec2,
    ctrl1: Vec2,
    ctrl2: Vec2,
    to: Vec2,
}
impl CurveCubicBezier {
    pub fn to_cubic(&self) -> CubicBezier {
        CubicBezier {
            from: self.from,
            ctrl1: self.ctrl1,
            ctrl2: self.ctrl2,
            to: self.to,
        }
    }

    pub fn from_cubic(c: CubicBezier) -> Self {
        Self {
            from: c.from,
            ctrl1: c.ctrl1,
            ctrl2: c.ctrl2,
            to: c.to,
        }
    }

    pub fn first_point(&self) -> Vec2 {
        self.from
    }

    pub fn last_point(&self) -> Vec2 {
        self.to
    }

    pub fn reverse(&self) -> CurveCubicBezier {
        Self::from_cubic(self.to_cubic().reverse())
    }

    pub fn as_points(&self) -> CurvePoints {
        CurvePoints::new(self.to_cubic().to_vec2())
    }

    pub fn ctrl1(&self) -> Vec2 {
        self.ctrl1
    }

    pub fn ctrl2(&self) -> Vec2 {
        self.ctrl2
    }

    pub fn to(&self) -> Vec2 {
        self.to
    }

    pub fn flatten(&self, tolerance: f32) -> Vec<Vec2> {
        flatten_cubic_bezier_path_with_tolerance(&vec![self.to_cubic()], false, tolerance).unwrap()
    }

    pub fn to_pts(&self, tolerance: f32) -> CurveSegment {
        CurveSegment::new_simple_points(self.flatten(tolerance))
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct CurvePoints {
    #[lerpable(func = "lerpify_vec_vec2")]
    pub points: Vec<Vec2>,
}
impl CurvePoints {
    pub fn new(points: Vec<Vec2>) -> Self {
        assert!(!points.is_empty());
        Self { points }
    }

    pub fn first_point(&self) -> Vec2 {
        *self.points.first().unwrap()
    }

    pub fn last_point(&self) -> Vec2 {
        *self.points.last().unwrap()
    }

    pub fn points(&self) -> &Vec<Vec2> {
        &self.points
    }

    pub fn reverse(&self) -> Self {
        CurvePoints::new(self.points.iter().cloned().rev().collect_vec())
    }
}

pub trait ToCurveSegment {
    fn to_segment(&self) -> CurveSegment;
}

impl ToCurveSegment for CubicBezier {
    fn to_segment(&self) -> CurveSegment {
        CurveSegment::cubic(*self)
    }
}

impl ToCurveSegment for Vec2 {
    fn to_segment(&self) -> CurveSegment {
        CurveSegment::new_simple_point(*self)
    }
}

impl ToCurveSegment for Vec<Vec2> {
    fn to_segment(&self) -> CurveSegment {
        CurveSegment::new_simple_points(self.clone())
    }
}

impl ToCurveSegment for Polyline {
    fn to_segment(&self) -> CurveSegment {
        CurveSegment::new_simple_points(self.clone_to_vec())
    }
}

impl ToCurveSegment for Circle {
    fn to_segment(&self) -> CurveSegment {
        CurveSegment::new_simple_circle(self.center, self.radius)
    }
}

impl ToCurveSegment for CurveArc {
    fn to_segment(&self) -> CurveSegment {
        CurveSegment::Arc(self.clone())
    }
}

pub trait ToCurveDrawer {
    fn to_segments(&self) -> Vec<CurveSegment>;
    fn to_cd_closed(&self) -> CurveDrawer {
        CurveDrawer::new(self.to_segments(), true)
    }
    fn to_cd_open(&self) -> CurveDrawer {
        CurveDrawer::new(self.to_segments(), false)
    }
}

impl ToCurveDrawer for CurveSegment {
    fn to_segments(&self) -> Vec<CurveSegment> {
        vec![self.clone()]
    }
}

impl<T> ToCurveDrawer for T
where
    T: ToCurveSegment,
{
    fn to_segments(&self) -> Vec<CurveSegment> {
        self.to_segment().to_segments()
    }
}

impl ToCurveDrawer for Vec<CurveSegment> {
    fn to_segments(&self) -> Vec<CurveSegment> {
        self.clone()
    }
}

impl<T> ToCurveDrawer for Option<T>
where
    T: ToCurveDrawer,
{
    fn to_segments(&self) -> Vec<CurveSegment> {
        match self {
            Some(t) => t.to_segments(),
            None => vec![],
        }
    }
}

// impl ToCurveDrawer for Vec<Vec2> {
//     fn to_segments(&self) -> Vec<CurveSegment> {
//         vec![CurveSegment::new_simple_points(self.clone())]
//     }
// }

impl ToCurveDrawer for Vec<SpotOnCurve> {
    fn to_segments(&self) -> Vec<CurveSegment> {
        vec![CurveSegment::new_simple_points(
            self.iter().map(|x| x.loc()).collect_vec(),
        )]
    }
}

#[macro_export]
macro_rules! curve_segments {
    ($($expr:expr),* $(,)?) => {{
        let mut v: Vec<murrelet_draw::curve_drawer::CurveSegment> = Vec::new();
        $(
            v.extend($expr.to_segments());
        )*
        v
    }};
}
