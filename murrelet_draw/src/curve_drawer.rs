use glam::*;
use itertools::Itertools;
use lerpable::Lerpable;
use murrelet_common::*;
use murrelet_livecode_derive::*;

use crate::livecodetypes::anglepi::*;

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct CurveDrawer {
    pub segments: Vec<CurveSegment>,
    pub closed: bool, // this is mostly used for algorithms that use curve drawers. you'll need to use a style that's closed
}

impl CurveDrawer {
    pub fn new(segments: Vec<CurveSegment>, closed: bool) -> Self {
        Self { segments, closed }
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
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub enum CurveSegment {
    Arc(CurveArc),
    Points(CurvePoints),
    // Bezier(CubicBezier),
}

impl CurveSegment {
    pub fn first_point(&self) -> Vec2 {
        match self {
            CurveSegment::Arc(c) => c.first_point(),
            CurveSegment::Points(c) => c.first_point(),
            // CurveSegment::Bezier(_) => todo!(),
        }
    }

    pub fn last_point(&self) -> Vec2 {
        match self {
            CurveSegment::Arc(c) => c.last_point(),
            CurveSegment::Points(c) => c.last_point(),
        }
    }

    pub fn reverse(&self) -> Self {
        match self {
            CurveSegment::Arc(curve_arc) => CurveSegment::Arc(curve_arc.reverse()),
            CurveSegment::Points(curve_points) => CurveSegment::Points(curve_points.reverse()),
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
        }
    }

    pub fn first_angle(&self) -> Option<AnglePi> {
        match self {
            CurveSegment::Arc(curve_arc) => {
                Some(curve_arc.start_angle().perp_to_left().as_angle_pi())
            }
            CurveSegment::Points(_) => None,
        }
    }

    pub fn first_spot(&self) -> SpotOnCurve {
        SpotOnCurve::new(self.first_point(), self.first_angle().unwrap())
    }

    pub fn last_angle(&self) -> Option<AnglePi> {
        match self {
            CurveSegment::Arc(curve_arc) => {
                Some(curve_arc.end_angle().perp_to_right().as_angle_pi())
            }
            CurveSegment::Points(_) => None,
        }
    }

    pub fn last_spot(&self) -> SpotOnCurve {
        SpotOnCurve::new(self.last_point(), self.last_angle().unwrap())
    }
}

#[derive(Debug, Clone, Livecode, Lerpable)]
pub struct CurveArc {
    #[livecode(serde_default = "zeros")]
    #[lerpable(func = "lerpify_vec2")]
    pub loc: Vec2, // center of circle
    pub radius: f32,
    pub start_pi: LivecodeAnglePi,
    pub end_pi: LivecodeAnglePi,
}
impl CurveArc {
    pub fn new<A1: IsAngle, A2: IsAngle>(loc: Vec2, radius: f32, start_pi: A1, end_pi: A2) -> Self {
        Self {
            loc,
            radius,
            start_pi: LivecodeAnglePi::new(start_pi),
            end_pi: LivecodeAnglePi::new(end_pi),
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
        (self.end_pi.angle_pi() - self.start_pi.angle_pi()).abs() > 1.0
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
