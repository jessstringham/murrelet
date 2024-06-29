use std::f32::consts::PI;

use glam::*;
use murrelet_common::{Circle, IsAngle, IsLength};
use murrelet_livecode_derive::*;

#[derive(Debug, Clone, UnitCell, Default, Livecode)]
pub struct CurveDrawer {
    segments: Vec<CurveSegment>,
}

impl CurveDrawer {
    pub fn new(segments: Vec<CurveSegment>) -> Self {
        Self { segments }
    }

    pub fn segments(&self) -> &[CurveSegment] {
        &self.segments
    }

    pub fn add_segment(&mut self, cs: CurveSegment) {
        self.segments.push(cs);
    }

    pub fn new_simple_arc<A: IsAngle>(loc: Vec2, radius: f32, start: A, end: A) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_arc(loc, radius, start, end)])
    }

    pub fn new_simple_sector<A: IsAngle>(loc: Vec2, radius: f32, start: A, end: A) -> Self {
        CurveDrawer::new(vec![
            CurveSegment::new_simple_point(loc),
            CurveSegment::Arc(CurveArc::new(loc, radius, start.angle_pi(), end.angle_pi())),
        ])
    }

    pub fn new_simple_circle(loc: Vec2, radius: f32) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_circle(loc, radius)])
    }

    pub fn new_from_circle(c: &Circle) -> Self {
        CurveDrawer::new_simple_circle(c.center, c.radius)
    }

    pub fn new_simple_line(start: Vec2, end: Vec2) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_line(start, end)])
    }

    pub fn new_simple_points(vs: Vec<Vec2>) -> Self {
        CurveDrawer::new(vec![CurveSegment::new_simple_points(vs)])
    }
}

#[derive(Debug, Clone, UnitCell, Livecode)]
pub enum CurveSegment {
    Arc(CurveArc),
    Points(CurvePoints),
}

impl CurveSegment {
    pub fn first_point(&self) -> Vec2 {
        match self {
            CurveSegment::Arc(c) => c.first_point(),
            CurveSegment::Points(c) => c.first_point(),
        }
    }

    pub fn last_point(&self) -> Vec2 {
        match self {
            CurveSegment::Arc(c) => c.last_point(),
            CurveSegment::Points(c) => c.last_point(),
        }
    }

    pub fn new_simple_arc<Rad: IsLength, A1: IsAngle, A2: IsAngle>(
        loc: Vec2,
        radius: Rad,
        start: A1,
        end: A2,
    ) -> Self {
        CurveSegment::Arc(CurveArc::new(
            loc,
            radius.len(),
            start.angle_pi(),
            end.angle_pi(),
        ))
    }

    pub fn new_simple_circle(loc: Vec2, radius: f32) -> Self {
        CurveSegment::Arc(CurveArc::new(loc, radius, 0.0, 2.0))
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
}

#[derive(Debug, Clone, UnitCell, Livecode, Default)]
pub struct CurveArc {
    #[livecode(serde_default = "zeros")]
    pub loc: Vec2, // center of circle
    pub radius: f32,
    pub start_pi: f32,
    pub end_pi: f32,
}
impl CurveArc {
    pub fn new(loc: Vec2, radius: f32, start_pi: f32, end_pi: f32) -> Self {
        Self {
            loc,
            radius,
            start_pi,
            end_pi,
        }
    }

    pub fn is_ccw(&self) -> bool {
        self.end_pi > self.start_pi
    }

    // useful for svg
    pub fn is_large_arc(&self) -> bool {
        (self.end_pi - self.start_pi).abs() > 1.0
    }

    pub fn last_point(&self) -> Vec2 {
        let curr_angle = self.end_pi * PI;
        let (loc_sin, loc_cos) = curr_angle.sin_cos();
        vec2(loc_cos, loc_sin) * self.radius + self.loc
    }

    pub fn first_point(&self) -> Vec2 {
        let curr_angle = self.start_pi * PI;
        let (loc_sin, loc_cos) = curr_angle.sin_cos();
        vec2(loc_cos, loc_sin) * self.radius + self.loc
    }
}

#[derive(Debug, Clone, UnitCell, Livecode, Default)]
pub struct CurvePoints {
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
}
