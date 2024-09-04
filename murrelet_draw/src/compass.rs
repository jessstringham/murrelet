#![allow(dead_code)]
use glam::Vec2;
use murrelet_common::*;
use murrelet_livecode_derive::Livecode;

use crate::curve_drawer::{CurveArc, CurveDrawer, CurvePoints, CurveSegment};

#[derive(Debug, Clone, Copy, Livecode, Default)]
pub struct CurveStart {
    loc: Vec2,
    angle: f32,
}

impl CurveStart {
    pub fn new(loc: Vec2, angle: f32) -> Self {
        Self { loc, angle }
    }
}

fn empty_string() -> String {
    String::new()
}

#[derive(Debug, Clone, Livecode, Default)]
pub struct CompassDir {
    angle: f32,
    #[livecode(serde_default = "false")]
    is_absolute: bool,
    #[livecode(serde_default = "empty_string")]
    label: String,
}

#[derive(Debug, Clone, Livecode, Default)]
pub struct CompassArc {
    radius: f32,
    arc_length: f32,
    #[livecode(serde_default = "false")]
    is_absolute: bool,
    #[livecode(serde_default = "empty_string")]
    label: String,
}

// impl CompassArc {
//     pub fn new(radius: f32, arc_length: f32, is_absolute: bool) -> Self { Self { radius, arc_length, is_absolute } }
// }

#[derive(Debug, Clone, Livecode, Default)]
pub struct CompassLine {
    length: f32, // how far should we head in the current direction
    #[livecode(serde_default = "empty_string")]
    label: String,
}

#[derive(Debug, Clone, Livecode, Default)]
pub struct CompassRepeat {
    times: usize,
    what: Vec<CompassAction>,
}

#[derive(Debug, Clone, Livecode)]
pub enum CompassAction {
    Angle(CompassDir), // abs
    Arc(CompassArc),
    Line(CompassLine),
    Repeat(CompassRepeat),
}

impl CompassAction {
    pub fn qangle(angle: f32) -> CompassAction {
        CompassAction::angle(angle, false, "".to_string())
    }

    pub fn qarc<A: IsAngle>(radius: f32, arc_length: A) -> CompassAction {
        CompassAction::arc(radius, arc_length.angle_pi(), false, "".to_string())
    }

    pub fn qline(length: f32) -> CompassAction {
        CompassAction::line(length, "".to_string())
    }

    pub fn angle(angle: f32, is_absolute: bool, label: String) -> CompassAction {
        CompassAction::Angle(CompassDir {
            angle,
            is_absolute,
            label,
        })
    }

    pub fn arc(radius: f32, arc_length: f32, is_absolute: bool, label: String) -> CompassAction {
        CompassAction::Arc(CompassArc {
            radius,
            arc_length,
            is_absolute,
            label,
        })
    }

    pub fn line(length: f32, label: String) -> CompassAction {
        CompassAction::Line(CompassLine { length, label })
    }

    pub fn repeat(times: usize, what: Vec<CompassAction>) -> CompassAction {
        CompassAction::Repeat(CompassRepeat { times, what })
    }
}
impl Default for CompassAction {
    fn default() -> Self {
        CompassAction::Angle(CompassDir {
            angle: 0.0,
            is_absolute: false,
            label: String::new(),
        })
    }
}

pub struct InteractiveCompassBuilder {
    pen_down: bool, // if we've drawn one
    curr_loc: Vec2,
    curr_angle: f32,
    so_far: Vec<CurveSegment>,
}

impl InteractiveCompassBuilder {
    pub fn new() -> Self {
        Self {
            pen_down: false,
            curr_loc: Vec2::ZERO,
            curr_angle: 0.0,
            so_far: Vec::new(),
        }
    }

    pub fn add_curve_start_simple(&mut self, loc: Vec2, angle: f32) {
        self.set_curr_angle(angle);
        self.set_curr_loc(loc);
    }

    pub fn add_curve_start(&mut self, start: CurveStart) {
        self.add_curve_start_simple(start.loc, start.angle);
    }

    pub fn new_segments(&mut self, dir: &CompassAction) -> Vec<CurveSegment> {
        // here we go!
        match dir {
            CompassAction::Angle(x) => {
                self.set_angle(x);
                vec![]
            }
            CompassAction::Line(x) => {
                vec![self.add_line(x)]
            }
            CompassAction::Arc(x) => {
                vec![self.add_arc(x)]
            }
            CompassAction::Repeat(x) => {
                let mut n = Vec::new();
                for _ in 0..x.times {
                    for w in &x.what {
                        n.extend(self.new_segments(w))
                    }
                }
                n
            }
        }
    }

    pub fn add_segment(&mut self, dir: &CompassAction) {
        let r = { self.new_segments(dir) };
        self.so_far.extend(r);
    }

    fn set_angle(&mut self, dir: &CompassDir) {
        if dir.is_absolute {
            self.curr_angle = dir.angle;
        } else {
            self.curr_angle += dir.angle;
        }
    }

    fn to_basic(&self) -> CurveStart {
        CurveStart {
            loc: self.curr_loc,
            angle: self.curr_angle,
        }
    }

    fn add_line(&mut self, x: &CompassLine) -> CurveSegment {
        // if the pen is not down, add the current spot
        let mut points = vec![];
        if !self.pen_down {
            points.push(self.curr_loc)
        }

        // next point is going to take the current angle, and move in that direction
        let movement = self.use_angle_and_length(self.curr_angle, x.length);
        self.curr_loc += movement;

        points.push(self.curr_loc);

        self.pen_down = true;

        // trying granular to see if we can mask
        CurveSegment::Points(CurvePoints::new(points))
    }

    fn use_angle_and_length(&self, angle: f32, length: f32) -> Vec2 {
        AnglePi::new(angle).to_norm_dir() * length
    }

    fn add_arc(&mut self, x: &CompassArc) -> CurveSegment {
        let (arc_length, radius) = if x.arc_length < 0.0 {
            (-x.arc_length, -x.radius)
        } else {
            (x.arc_length, x.radius)
        };

        // starting at our current location, move at a right angle to our current angle
        // negative goes to the left of the line
        let loc = self.curr_loc + self.use_angle_and_length(self.curr_angle + 0.5, radius);

        // if radius is negative, go backwards
        // end_angle is what we'll update curr angle to, it's always assuming positive radius
        let (start, end, next_angle) = if radius < 0.0 {
            let next_angle = self.curr_angle - arc_length;
            (
                1.0 + self.curr_angle - 0.5,
                1.0 + next_angle - 0.5,
                next_angle,
            )
        } else {
            let next_angle = self.curr_angle + arc_length;
            (self.curr_angle - 0.5, next_angle - 0.5, next_angle)
        };

        let a = CurveArc::new(loc, radius.abs(), start, end);

        self.curr_loc = a.last_point();
        self.curr_angle = next_angle % 2.0;
        self.pen_down = true;

        CurveSegment::Arc(a)
    }

    pub fn results(&self) -> Vec<CurveSegment> {
        self.so_far.clone()
    }

    pub fn curr_loc(&self) -> Vec2 {
        self.curr_loc
    }

    pub fn curr_angle(&self) -> f32 {
        self.curr_angle
    }

    pub fn set_curr_loc(&mut self, curr_loc: Vec2) {
        self.curr_loc = curr_loc;
    }

    pub fn set_curr_angle(&mut self, curr_angle: f32) {
        self.curr_angle = curr_angle;
    }
}

#[derive(Debug, Clone, Livecode, Default)]
pub struct MurreletCompass {
    start: CurveStart,
    dirs: Vec<CompassAction>,
    closed: bool,
}

impl MurreletCompass {
    pub fn new(start: CurveStart, dirs: Vec<CompassAction>, closed: bool) -> Self {
        Self {
            start,
            dirs,
            closed,
        }
    }

    pub fn to_curve_maker(&self) -> CurveDrawer {
        let mut builder = InteractiveCompassBuilder::new();

        let start = self.start;
        builder.add_curve_start(start);
        for w in self.dirs.iter() {
            builder.add_segment(&w)
        }

        CurveDrawer::new(builder.results(), self.closed)
    }
}
