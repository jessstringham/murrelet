use crate::{
    convert::{glam_to_kurbo, glam_to_lyon},
    cubic::CubicBezier,
    curve_drawer::{CubicBezierPath, CurveArc},
    svg_read::parse_svg_data_as_vec2,
};
use glam::{Vec2, Vec2Swizzles, vec2};
use itertools::Itertools;
use kurbo::BezPath;
use murrelet_common::{AnglePi, IsAngle, PointToPoint, SpotOnCurve, curr_next_no_loop_iter};

pub trait ToVecVec2 {
    fn to_vec2(&self) -> Vec<Vec2>;

    fn to_vec2_line_space(&self, _line_space: f32) -> Vec<Vec2> {
        todo!()
    }

    fn to_vec2_count(&self, _count: usize) -> Vec<Vec2> {
        todo!()
    }
}

impl ToVecVec2 for CubicBezier {
    fn to_vec2_line_space(&self, line_space: f32) -> Vec<Vec2> {
        let mut svg = svg::node::element::path::Data::new();

        let x = self.from.x;
        let y = self.from.y;
        let start: svg::node::element::path::Parameters = vec![x, y].into();
        svg = svg.move_to(start);

        let cubic: svg::node::element::path::Parameters = vec![
            self.ctrl1.x,
            self.ctrl1.y,
            self.ctrl2.x,
            self.ctrl2.y,
            self.to.x,
            self.to.y,
        ]
        .into();
        svg = svg.cubic_curve_to(cubic);

        let mut path = parse_svg_data_as_vec2(&svg, line_space);

        if let Some(a) = path.last()
            && a.distance(self.to.yx()) > 1.0e-3
        {
            path.push(self.to.yx())
        }

        path.into_iter().map(|x| vec2(x.y, x.x)).collect_vec()
    }

    fn to_vec2(&self) -> Vec<Vec2> {
        self.to_vec2_line_space(1.0)
    }
}

impl ToVecVec2 for CubicBezierPath {
    fn to_vec2(&self) -> Vec<Vec2> {
        let svg = self.to_data();
        let path = parse_svg_data_as_vec2(&svg, 1.0);

        path.into_iter().map(|x| vec2(x.y, x.x)).collect_vec()
    }
}

pub fn cubic_bezier_length(c: &CubicBezier) -> f32 {
    let line = lyon::geom::CubicBezierSegment {
        from: glam_to_lyon(c.from),
        ctrl1: glam_to_lyon(c.ctrl1),
        ctrl2: glam_to_lyon(c.ctrl2),
        to: glam_to_lyon(c.to),
    };
    line.approximate_length(0.1)
}

pub fn segment_vec(from: Vec2, to: Vec2, line_space: f32, offset: f32) -> (Vec<Vec2>, f32) {
    if to.distance(from) < 1e-7 {
        return (vec![], offset);
    }

    let mut dist_since_last = offset; // how far into this one we should start
    dist_since_last += (to - from).length(); // add how far we will travel

    let backwards_dir = (to - from).normalize();

    let mut lines = vec![];

    while dist_since_last >= line_space {
        // okay find how much we overshot it, and move backwards
        // it should be within to and from.. or else we would've stopped before?
        let overshot_amount = dist_since_last - line_space;
        // figure out how to go backwards
        // and now move backwards
        let new_to = to - backwards_dir * overshot_amount;
        lines.push(new_to);
        dist_since_last = overshot_amount;
    }

    (lines, dist_since_last)
}

pub fn segment_arc(
    curve: &CurveArc,
    height: f32,
    line_space: f32,
    offset: f32,
) -> (Vec<SpotOnCurve>, f32) {
    // going into this curve, we should be all caught up
    let multi = if curve.is_ccw() { 1.0 } else { -1.0 };
    let radius = curve.radius.abs();

    let increase_ratio = (radius + 0.5 * height) / radius;
    let mut line_space = line_space / increase_ratio;

    // make sure we always have at least a few points
    let max_delta_angle = AnglePi::new(0.1);
    line_space = line_space.min(radius * max_delta_angle._angle());

    // expect line_space to be > 0
    if line_space <= 0.0 {
        return (vec![], offset);
    }

    // okay! now if we already traveled some distance along this curve (e.g. dist_since_last > 0)
    // we need to remove some of it
    let diameter = AnglePi::new(2.0).scale(radius).angle();

    let estimated_size = (diameter / line_space) as usize;
    let mut vs = Vec::with_capacity(estimated_size);

    let mut residual = offset + curve.length();

    // the last point was shifted back
    let mut loc_on_arc = -offset;

    while residual >= line_space {
        residual -= line_space;
        loc_on_arc += line_space;

        let central_angle_from_start = AnglePi::new(2.0).scale(multi * loc_on_arc / diameter);
        let curr_angle = curve.start_pi() + central_angle_from_start;

        let norm_vec = curr_angle.to_norm_dir();
        let curr_point = norm_vec * radius + curve.loc;

        let a = if curve.is_ccw() {
            curr_angle.perp_to_right()
        } else {
            curr_angle.perp_to_left()
        };

        let s = SpotOnCurve::new(curr_point, a);
        vs.push(s);
    }

    (vs, residual / increase_ratio)
}

pub fn evenly_split_cubic_bezier(c: &CubicBezier, count: usize) -> Vec<SpotOnCurve> {
    // always include the start (and end)
    let v = flatten_cubic_bezier_path_with_tolerance(&[*c], false, 0.1);
    if count <= 1 {
        let angle = PointToPoint::new(c.from, c.to).angle();
        return vec![c.from, c.to]
            .into_iter()
            .map(|x| SpotOnCurve::new(x, angle))
            .collect_vec();
    }

    if let Some(a) = v {
        let mut length = 0.0;
        for (curr, next) in curr_next_no_loop_iter(&a) {
            length += curr.distance(*next);
        }

        let spacing_for_dot = length / count as f32;

        let mut last_angle = None;

        let mut v = vec![];
        let mut dist = 0.0;
        for (curr, next) in curr_next_no_loop_iter(&a) {
            let (vs, a) = segment_vec(*curr, *next, spacing_for_dot, dist);
            let angle = PointToPoint::new(*curr, *next).angle();

            // if it's the very first item
            if last_angle.is_none() {
                v.push(SpotOnCurve::new(*curr, angle))
            }

            let new = vs
                .into_iter()
                .map(|x| SpotOnCurve::new(x, angle))
                .collect_vec();
            v.extend(new);
            dist = a;

            last_angle = Some(angle);
        }

        v
    } else {
        let angle = PointToPoint::new(c.from, c.to).angle();
        vec![c.from, c.to]
            .into_iter()
            .map(|x| SpotOnCurve::new(x, angle))
            .collect_vec()
    }
}

pub fn flatten_cubic_bezier_path_with_tolerance(
    path: &[CubicBezier],
    closed: bool,
    tolerance: f32,
) -> Option<Vec<Vec2>> {
    if path.is_empty() {
        return None;
    }
    let mut kurbo_path = BezPath::new();

    kurbo_path.move_to(glam_to_kurbo(path[0].from));
    for c in path {
        kurbo_path.curve_to(
            glam_to_kurbo(c.ctrl1),
            glam_to_kurbo(c.ctrl2),
            glam_to_kurbo(c.to),
        )
    }

    if closed {
        kurbo_path.close_path();
    }

    let mut points = Vec::new();
    kurbo::flatten(kurbo_path, tolerance as f64, |el| match el {
        kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => {
            points.push(vec2(p.x as f32, p.y as f32));
        }
        kurbo::PathEl::ClosePath => {
            if let Some(first) = points.first() {
                points.push(*first);
            }
        }
        _ => {}
    });

    Some(points)
}

pub fn flatten_cubic_bezier_path(path: &[CubicBezier], closed: bool) -> Option<Vec<Vec2>> {
    flatten_cubic_bezier_path_with_tolerance(path, closed, 0.01)
}
