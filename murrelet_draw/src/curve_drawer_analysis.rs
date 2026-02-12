use crate::{
    cubic::CubicBezier,
    curve_drawer::*,
    drawable::{DrawnShape, ToDrawnShape},
    style::styleconf::StyleConf,
};
use glam::Vec2;
use lerpable::Lerpable;
use murrelet_common::*;

pub struct CurveDrawerDebugStyle {
    cb_fill: StyleConf,
    cb_line: StyleConf,
    arc_line: StyleConf,
    points: StyleConf,
    arc_centers: StyleConf,
    start_point: StyleConf,
    end_point: StyleConf,
}
impl Default for CurveDrawerDebugStyle {
    fn default() -> Self {
        let fill = StyleConf::outlined_fill(MurreletColor::black(), 0.25, MurreletColor::white());
        let start = StyleConf::fill(MurreletColor::hsva(0.4, 1.0, 1.0, 1.0));
        let end = StyleConf::fill(MurreletColor::hsva(0.0, 1.0, 1.0, 1.0));
        let line = StyleConf::line(MurreletColor::hsva(0.5, 0.3, 1.0, 0.5), 0.25);
        let arcline = StyleConf::line(MurreletColor::hsva(0.2, 0.3, 1.0, 0.5), 0.25);
        Self {
            cb_fill: fill.clone(),
            cb_line: line.clone(),
            arc_line: arcline.clone(),
            points: fill.clone(),
            arc_centers: fill.clone(),
            start_point: start,
            end_point: end,
        }
    }
}

pub struct CurveDrawerDebugShape {
    pub cb_dots: Vec<Vec2>,
    pub cb_ctrl_lines: Vec<PointToPoint>,
    pub arc_lines: Vec<PointToPoint>,
    points: Vec<Vec2>,
    arc_centers: Vec<Vec2>,
    start_marker: Vec<Vec<Vec2>>,
    end_marker: Vec<Vec<Vec2>>,
}

impl CurveDrawerDebugShape {
    pub fn new() -> Self {
        Self {
            cb_dots: vec![],
            cb_ctrl_lines: vec![],
            arc_lines: vec![],
            points: vec![],
            arc_centers: vec![],
            start_marker: vec![],
            end_marker: vec![],
        }
    }

    pub fn new_from_cd(cd: &CurveDrawer) -> Self {
        let mut d = Self::new();
        d.add_cd(cd);
        d
    }

    pub fn add_cd(&mut self, cd: &CurveDrawer) {
        for s in cd.segments() {
            match s {
                CurveSegment::Arc(c) => self.add_arc(c),
                CurveSegment::Points(c) => self.add_points(c.points()),
                CurveSegment::CubicBezier(c) => self.add_cubic_bezier(&c.to_cubic()),
            };
        }
    }

    fn add_end(&mut self, end: &SpotOnCurve, len: f32) {
        let end_point = end.move_back(len * 0.1);
        self.end_marker.push(vec![
            end_point.move_left_perp_dist(0.5),
            end_point.move_right_perp_dist(0.5),
            end_point.travel(1.0).loc,
        ]);
    }

    fn add_start(&mut self, start: &SpotOnCurve, len: f32) {
        let start_point = start.travel(len * 0.1);
        self.start_marker.push(vec![
            start_point.move_left_perp_dist(0.5),
            start_point.move_right_perp_dist(0.5),
            start_point.travel(-1.0).loc,
        ]);
    }

    pub fn to_drawn_shape(&self, style: &CurveDrawerDebugStyle) -> Vec<DrawnShape> {
        vec![
            self.cb_dots
                .map_iter_collect(|f| f.to_cd_open())
                .to_drawn_shape_r(&style.cb_fill),
            self.cb_ctrl_lines
                .map_iter_collect(|f| f.to_cd_open())
                .to_drawn_shape_r(&style.cb_line),
            self.arc_lines
                .map_iter_collect(|f| f.to_cd_open())
                .to_drawn_shape_r(&style.arc_line),
            self.arc_centers
                .map_iter_collect(|f| f.to_cd_open())
                .to_drawn_shape_r(&style.arc_centers),
            self.points
                .map_iter_collect(|f| f.to_cd_open())
                .to_drawn_shape_r(&style.points),
            self.start_marker
                .map_iter_collect(|f| f.to_cd_closed())
                .to_drawn_shape_r(&style.start_point),
            self.end_marker
                .map_iter_collect(|f| f.to_cd_closed())
                .to_drawn_shape_r(&style.end_point),
        ]
    }

    fn add_arc(&mut self, c: &CurveArc) {
        // we extend the arc a tiny bit
        self.arc_centers.push(c.loc);
        self.arc_lines
            .push(PointToPoint::new(c.loc, c.last_point()));
        self.arc_lines
            .push(PointToPoint::new(c.loc, c.first_point()));

        self.add_start(&c.first_spot(), c.length());
        self.add_end(&c.last_spot(), c.length());
    }

    fn add_points(&mut self, c: &[Vec2]) {
        self.points.extend(c.iter());

        for (curr, next) in curr_next_no_loop_iter(c) {
            let p2p = PointToPoint::new(*curr, *next);

            let len = curr.distance(*next);

            self.add_start(&p2p.start_spot(), len);
            self.add_end(&p2p.end_spot(), len);
        }
    }

    fn add_cubic_bezier(&mut self, cb: &CubicBezier) {
        self.cb_dots.push(cb.from);
        self.cb_dots.push(cb.to);

        self.cb_ctrl_lines
            .push(PointToPoint::new(cb.from, cb.ctrl1));
        self.cb_ctrl_lines.push(PointToPoint::new(cb.to, cb.ctrl2));

        let len = cb.start_spot().loc.distance(cb.end_spot().loc);

        self.add_start(&cb.start_spot(), len);
        self.add_end(&cb.end_spot(), len);
    }
}

// curvature brought to you by chatgpt

fn curvature_from_derivatives(dp: Vec2, ddp: Vec2) -> f32 {
    // κ = (x' y'' - y' x'') / ||p'||^3
    let cross = dp.x * ddp.y - dp.y * ddp.x;
    let speed2 = dp.length_squared();
    let denom = (speed2.sqrt() * speed2).max(1e-12);
    cross / denom
}

fn cubic_eval(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    (u * u * u) * p0 + (3.0 * u * u * t) * p1 + (3.0 * u * t * t) * p2 + (t * t * t) * p3
}

fn cubic_d1(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    3.0 * (u * u) * (p1 - p0) + 6.0 * (u * t) * (p2 - p1) + 3.0 * (t * t) * (p3 - p2)
}

fn cubic_d2(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    6.0 * u * (p2 - 2.0 * p1 + p0) + 6.0 * t * (p3 - 2.0 * p2 + p1)
}

/// Signed curvature for a circular arc: +1/R for CCW, -1/R for CW.
fn arc_signed_curvature(is_ccw: bool, radius: f32) -> f32 {
    let k = 1.0 / radius.max(1e-12);
    if is_ccw { k } else { -k }
}

/// Sample curvature along a CurveDrawer using an approximate spacing in world units.
/// - `approx_spacing`: e.g. 1.0, 2.0, etc. Smaller = more samples.
/// - returns (spot with tangent, signed curvature κ).
pub fn sample_curve_drawer_curvature(
    cd: &CurveDrawer,
    approx_spacing: f32,
) -> Vec<(SpotOnCurve, f32)> {
    let spacing = approx_spacing.max(1e-6);
    let mut out: Vec<(SpotOnCurve, f32)> = Vec::new();
    let mut last_angle: Option<Angle> = None;

    fn push_spot(out: &mut Vec<(SpotOnCurve, f32)>, spot: SpotOnCurve, k: f32) -> bool {
        // avoid duplicate boundary points
        if let Some((last, _)) = out.last() {
            if last.loc.distance(spot.loc) <= 1e-6 {
                return false;
            }
        }
        out.push((spot, k));
        true
    }

    for seg in cd.segments() {
        match seg {
            CurveSegment::Arc(a) => {
                let len = a.length().max(0.0);
                let steps = ((len / spacing).ceil() as usize).max(1);
                let k = arc_signed_curvature(a.is_ccw(), a.radius);

                for i in 0..=steps {
                    let t = i as f32 / steps as f32;

                    // linearly interpolate angle in AnglePi-space (good enough for sampling)
                    let ang_pi = a.start_pi.lerpify(&a.end_pi, &t);

                    let pt = ang_pi.to_norm_dir() * a.radius + a.loc;

                    let tangent_pi = if a.is_ccw() {
                        ang_pi + AnglePi::new(0.5)
                    } else {
                        ang_pi + AnglePi::new(-0.5)
                    };

                    let spot = SpotOnCurve::new(pt, tangent_pi);
                    let angle = spot.angle;
                    if push_spot(&mut out, spot, k) {
                        last_angle = Some(angle);
                    }
                }
            }

            CurveSegment::CubicBezier(cb) => {
                let c = cb.to_cubic();
                let len = cb.length().max(0.0);
                let steps = ((len / spacing).ceil() as usize).max(1);

                for i in 0..=steps {
                    let t = i as f32 / steps as f32;

                    let p = cubic_eval(c.from, c.ctrl1, c.ctrl2, c.to, t);
                    let dp = cubic_d1(c.from, c.ctrl1, c.ctrl2, c.to, t);
                    let ddp = cubic_d2(c.from, c.ctrl1, c.ctrl2, c.to, t);

                    let k = curvature_from_derivatives(dp, ddp);

                    let mut tangent_vec = dp;
                    if tangent_vec.length_squared() <= 1e-10 {
                        tangent_vec = ddp;
                    }

                    let spot = if tangent_vec.length_squared() <= 1e-10 {
                        SpotOnCurve::new(p, last_angle.unwrap_or_else(|| AnglePi::new(0.0).into()))
                    } else {
                        SpotOnCurve::new(p, Angle::new(tangent_vec.to_angle()))
                    };

                    let angle = spot.angle;
                    if push_spot(&mut out, spot, k) {
                        last_angle = Some(angle);
                    }
                }
            }

            CurveSegment::Points(p) => {
                // Curvature along a straight segment is 0; corners are undefined,
                // so we just output 0 for these samples.
                let pts = p.points();
                if pts.len() == 1 {
                    let spot = SpotOnCurve::new(
                        pts[0],
                        last_angle.unwrap_or_else(|| AnglePi::new(0.0).into()),
                    );
                    let angle = spot.angle;
                    if push_spot(&mut out, spot, 0.0) {
                        last_angle = Some(angle);
                    }
                } else {
                    for w in pts.windows(2) {
                        let a = w[0];
                        let b = w[1];
                        let len = a.distance(b).max(0.0);
                        let steps = ((len / spacing).ceil() as usize).max(1);
                        let angle = PointToPoint::new(a, b).angle();

                        for i in 0..=steps {
                            let t = i as f32 / steps as f32;
                            let spot = SpotOnCurve::new(a.lerp(b, t), angle);
                            if push_spot(&mut out, spot, 0.0) {
                                last_angle = Some(angle);
                            }
                        }
                    }
                }
            }
        }
    }

    out
}
