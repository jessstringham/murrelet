#![allow(dead_code)]
use std::collections::HashMap;

use glam::Vec2;
use glam::vec2;
use itertools::Itertools;
use lerpable::Lerpable;
use murrelet_common::*;
use murrelet_gui::MurreletGUI;
use murrelet_gui::MurreletGUISchema;
use murrelet_gui::ValueGUI;
use murrelet_gui::make_gui_angle;
use murrelet_gui::make_gui_vec2;
use murrelet_livecode_derive::Livecode;

use crate::cubic::CubicBezier;
use crate::curve_drawer::ToCurveSegment;
use crate::curve_drawer::{CurveArc, CurveDrawer, CurvePoints, CurveSegment};

#[derive(Debug, Clone, Copy, Livecode, MurreletGUI, Lerpable)]
pub struct CurveStart {
    #[murrelet_gui(func = "make_gui_vec2")]
    loc: Vec2,
    #[murrelet_gui(func = "make_gui_angle")]
    angle_pi: AnglePi,
}

impl CurveStart {
    pub fn new<A: IsAngle>(loc: Vec2, angle: A) -> Self {
        Self {
            loc,
            angle_pi: angle.as_angle_pi(),
        }
    }
}

fn empty_string() -> String {
    String::new()
}

#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
pub struct CompassDir {
    #[murrelet_gui(func = "make_gui_angle")]
    pub angle_pi: AnglePi,
    #[livecode(serde_default = "false")]
    #[murrelet_gui(kind = "skip")]
    pub is_absolute: bool,
    #[livecode(serde_default = "murrelet_livecode::livecode::empty_string")]
    #[murrelet_gui(kind = "skip")]
    pub label: String,
}

impl CompassDir {
    pub fn new<A: IsAngle>(angle: A, is_absolute: bool, label: String) -> Self {
        Self {
            angle_pi: angle.as_angle_pi(),
            is_absolute,
            label,
        }
    }
}

#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
pub struct CompassArc {
    pub radius: f32,
    #[murrelet_gui(func = "make_gui_angle")]
    pub arc_length: AnglePi,
    #[livecode(serde_default = "false")]
    #[murrelet_gui(kind = "skip")]
    pub is_absolute: bool,
    #[livecode(serde_default = "murrelet_livecode::livecode::empty_string")]
    #[murrelet_gui(kind = "skip")]
    pub label: String,
}

#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
pub struct CompassBezier {
    dist: f32,
    #[murrelet_gui(func = "make_gui_angle")]
    angle: AnglePi,
    #[murrelet_gui(func = "make_gui_vec2")]
    strengths: Vec2,
    #[livecode(serde_default = "murrelet_livecode::livecode::empty_string")]
    #[murrelet_gui(kind = "skip")]
    pub label: String,
}

#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
pub struct CompassLine {
    pub length: f32, // how far should we head in the current direction
    #[livecode(serde_default = "murrelet_livecode::livecode::empty_string")]
    #[murrelet_gui(kind = "skip")]
    pub label: String,
}

pub fn make_gui_vec_vec2() -> MurreletGUISchema {
    MurreletGUISchema::list(MurreletGUISchema::Val(ValueGUI::Vec2))
}

#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
pub struct CompassAbsPoints {
    #[lerpable(func = "lerpify_vec_vec2")]
    #[murrelet_gui(func = "make_gui_vec_vec2")]
    pts: Vec<Vec2>,
    #[livecode(serde_default = "murrelet_livecode::livecode::empty_string")]
    #[murrelet_gui(kind = "skip")]
    label: String,
}

#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
pub enum CompassAction {
    Angle(CompassDir), // abs
    Arc(CompassArc),
    Line(CompassLine),
    Bezier(CompassBezier),
    AbsPoints(CompassAbsPoints), // Repeat(CompassRepeat), // now this is in the control vec!
}

impl CompassAction {
    pub fn qabspts(pts: &[Vec2]) -> CompassAction {
        CompassAction::abspts(pts, "".to_string())
    }

    pub fn abspts(pts: &[Vec2], label: String) -> CompassAction {
        CompassAction::AbsPoints(CompassAbsPoints {
            pts: pts.to_vec(),
            label,
        })
    }

    pub fn qangle<A: IsAngle>(angle_pi: A) -> CompassAction {
        CompassAction::angle(angle_pi, false, "".to_string())
    }

    pub fn qarc<A: IsAngle>(radius: f32, arc_length: A) -> CompassAction {
        CompassAction::arc(radius, arc_length, false, "".to_string())
    }

    pub fn qline(length: f32) -> CompassAction {
        CompassAction::line(length, "".to_string())
    }

    pub fn angle<A: IsAngle>(angle_pi: A, is_absolute: bool, label: String) -> CompassAction {
        CompassAction::Angle(CompassDir {
            angle_pi: angle_pi.as_angle_pi(),
            is_absolute,
            label,
        })
    }

    pub fn arc<A: IsAngle>(
        radius: f32,
        arc_length_pi: A,
        is_absolute: bool,
        label: String,
    ) -> CompassAction {
        CompassAction::Arc(CompassArc {
            radius,
            arc_length: arc_length_pi.as_angle_pi(),
            is_absolute,
            label,
        })
    }

    pub fn line(length: f32, label: String) -> CompassAction {
        CompassAction::Line(CompassLine { length, label })
    }
}
impl Default for CompassAction {
    fn default() -> Self {
        CompassAction::Angle(CompassDir {
            angle_pi: AnglePi::ZERO,
            is_absolute: false,
            label: String::new(),
        })
    }
}

#[derive(Clone, Debug)]
pub enum LastActionNames {
    Arc,
    Line,
    Bezier,
    Start,
}
impl LastActionNames {
    pub fn from_segment(c: &CurveSegment) -> LastActionNames {
        match c {
            CurveSegment::Arc(_) => LastActionNames::Arc,
            CurveSegment::Points(_) => LastActionNames::Line,
            CurveSegment::CubicBezier(_) => LastActionNames::Bezier,
        }
    }
}

pub struct InteractiveCompassBuilder {
    pen_down: bool, // if we've drawn one
    curr_loc: Vec2,
    curr_angle: AnglePi,
    so_far: Vec<CurveSegment>,
    references: HashMap<String, CurveStart>,
    last_action: LastActionNames,
}

impl Default for InteractiveCompassBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractiveCompassBuilder {
    pub fn new() -> Self {
        Self {
            pen_down: false,
            curr_loc: Vec2::ZERO,
            curr_angle: AnglePi::new(0.0),
            so_far: Vec::new(),
            references: HashMap::new(),
            last_action: LastActionNames::Start,
        }
    }

    pub fn add_curve_start_simple<A: IsAngle>(&mut self, loc: Vec2, angle: A) {
        self.set_curr_angle(angle);
        self.set_curr_loc(loc);
    }

    pub fn add_curve_start(&mut self, start: CurveStart) {
        self.add_curve_start_simple(start.loc, start.angle_pi);
    }

    pub fn add_curve_start_spot(&mut self, spot: SpotOnCurve) {
        self.add_curve_start_simple(spot.loc, spot.angle);
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
            CompassAction::Bezier(x) => {
                vec![self.add_bezier(x)]
            }
            CompassAction::AbsPoints(x) => match self.add_abs_pts(x) {
                Some(x) => vec![x],
                None => vec![],
            },
        }
    }

    pub fn add_qline(&mut self, length: f32) {
        self.add_segment(&CompassAction::qline(length));
    }

    pub fn add_qangle<A: IsAngle>(&mut self, angle: A) {
        self.add_segment(&CompassAction::qangle(angle));
    }

    pub fn add_qarc<A: IsAngle>(&mut self, rad: f32, arc: A) {
        self.add_segment(&CompassAction::qarc(rad, arc));
    }

    pub fn add_segment(&mut self, dir: &CompassAction) {
        let r = self.new_segments(dir);
        self.so_far.extend(r);

        self.last_action = match dir {
            CompassAction::Angle(_) => self.last_action.clone(), // don't change the action
            CompassAction::Arc(_) => LastActionNames::Arc,
            CompassAction::Line(_) => LastActionNames::Line,
            CompassAction::Bezier(_) => LastActionNames::Bezier,
            CompassAction::AbsPoints(_) => LastActionNames::Line,
        }
    }

    fn set_angle(&mut self, dir: &CompassDir) {
        if dir.is_absolute {
            self.curr_angle = dir.angle_pi.as_angle_pi();
        } else {
            self.curr_angle = self.curr_angle + dir.angle_pi.as_angle_pi();
        }
    }

    fn to_basic(&self) -> CurveStart {
        CurveStart {
            loc: self.curr_loc,
            angle_pi: self.curr_angle.as_angle_pi(),
        }
    }

    fn add_line(&mut self, x: &CompassLine) -> CurveSegment {
        // if the pen is not down, add the current spot
        let mut points = vec![];
        if !self.pen_down {
            points.push(self.curr_loc)
        } else if !matches!(self.last_action, LastActionNames::Line) {
            points.push(self.curr_loc)
        }

        // next point is going to take the current angle, and move in that direction
        let movement = self.curr_angle.to_norm_dir() * x.length;
        self.curr_loc += movement;

        points.push(self.curr_loc);

        self.pen_down = true;

        if !x.label.is_empty() {
            self.references.insert(x.label.clone(), self.to_basic());
        }

        // know there's at least one point so we can unwrap
        CurveSegment::Points(CurvePoints::new(points))
    }

    fn curr_spot(&self) -> SpotOnCurve {
        SpotOnCurve::new(self.curr_loc, self.curr_angle)
    }

    fn add_bezier(&mut self, x: &CompassBezier) -> CurveSegment {
        let first_spot = self.curr_spot();

        // next point is going to take the current angle, and move in that direction
        let last_spot = first_spot.rotate(x.angle).travel(x.dist);

        let bezier =
            CubicBezier::from_spots_s(first_spot, last_spot, x.strengths * vec2(1.0, -1.0));

        self.curr_loc = last_spot.loc;
        self.curr_angle = last_spot.angle().as_angle_pi();

        self.pen_down = true;

        if !x.label.is_empty() {
            self.references.insert(x.label.clone(), self.to_basic());
        }
        bezier.to_segment()
    }

    fn add_arc(&mut self, x: &CompassArc) -> CurveSegment {
        let arc = CurveArc::from_spot(
            SpotOnCurve::new(self.curr_loc, self.curr_angle),
            x.radius,
            x.arc_length,
        );
        let new_spot = arc.last_spot();

        self.curr_loc = new_spot.loc;
        self.curr_angle = new_spot.angle().mod2();
        self.pen_down = true;

        if !x.label.is_empty() {
            self.references.insert(x.label.clone(), self.to_basic());
        }

        CurveSegment::Arc(arc)
    }

    fn add_abs_pts(&mut self, x: &CompassAbsPoints) -> Option<CurveSegment> {
        match x.pts.as_slice() {
            [.., penultimate, last] => {
                let pt = PointToPoint::new(*penultimate, *last).angle().as_angle_pi();
                self.curr_angle = pt;
                self.curr_loc = *last;
            }
            [last] => {
                if last.distance(self.curr_loc) > 0.001 {
                    let penultimate = self.curr_loc;
                    let pt = PointToPoint::new(penultimate, *last).angle().as_angle_pi();
                    self.curr_angle = pt;
                    self.curr_loc = *last;
                } else {
                    // not enough points to update the angle...
                    self.curr_loc = *last;
                }
            }
            _ => {
                return None; // not enough items
            }
        };

        self.pen_down = true;

        if !x.label.is_empty() {
            self.references.insert(x.label.clone(), self.to_basic());
        }

        Some(CurveSegment::Points(CurvePoints::new(x.pts.clone())))
    }

    pub fn results(&self) -> Vec<CurveSegment> {
        self.so_far.clone()
    }

    pub fn curr_loc(&self) -> Vec2 {
        self.curr_loc
    }

    pub fn curr_angle(&self) -> AnglePi {
        self.curr_angle
    }

    pub fn set_curr_loc(&mut self, curr_loc: Vec2) {
        self.curr_loc = curr_loc;
    }

    pub fn set_curr_angle<A: IsAngle>(&mut self, curr_angle: A) {
        self.curr_angle = curr_angle.as_angle_pi();
    }

    pub fn add_absolute_point(&mut self, loc: Vec2) {
        self.so_far
            .push(CurveSegment::Points(CurvePoints { points: vec![loc] }))
    }

    pub fn references(&self) -> Vec<(String, CurveStart)> {
        self.references.clone().into_iter().collect_vec()
    }
}

#[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable)]
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
            builder.add_segment(w)
        }

        CurveDrawer::new(builder.results(), self.closed)
    }
}

impl Default for MurreletCompass {
    fn default() -> Self {
        Self {
            start: CurveStart::new(Vec2::default(), AnglePi::new(0.0)),
            dirs: Default::default(),
            closed: Default::default(),
        }
    }
}
