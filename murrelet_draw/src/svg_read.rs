use std::{collections::HashMap, ops};

use crate::svg::SvgPathDef;
use glam::{Vec2, vec2};
use itertools::Itertools;
use lyon::geom::euclid::{Point2D, UnknownUnit};
use murrelet_common::Polyline;

fn _point_from_params(params: &Vec<&f32>, idx: usize) -> Pt {
    Pt::new(*params[idx * 2], *params[idx * 2 + 1])
}

fn point_from_param1(params: &Vec<&f32>) -> Pt {
    _point_from_params(params, 0)
}

fn point_from_param2(params: &Vec<&f32>) -> (Pt, Pt) {
    (_point_from_params(params, 0), _point_from_params(params, 1))
}

fn point_from_param3(params: &Vec<&f32>) -> (Pt, Pt, Pt) {
    (
        _point_from_params(params, 0),
        _point_from_params(params, 1),
        _point_from_params(params, 2),
    )
}

pub fn many_pt2_to_vec2(ps: &Vec<Pt>) -> Vec<Vec2> {
    ps.iter().map(|p| p.as_vec2()).collect_vec()
}

pub fn parse_svg_data_as_vec2(data: &svg::node::element::path::Data, line_space: f32) -> Vec<Vec2> {
    parse_data(data, line_space)
}

// svg loader
fn parse_data(data: &svg::node::element::path::Data, line_space: f32) -> Vec<Vec2> {
    let mut segment_state = SegmentState::new_with_line_space(line_space);

    let mut from = Pt::new(0.0, 0.0);

    // https://developer.mozilla.org/en-US/docs/Web/SVG/Tutorial/Paths
    for command in data.iter() {
        // println!("{:?}", command);

        match command {
            svg::node::element::path::Command::Move(_pos, params) => {
                let curve: Vec<&f32> = params.iter().collect();
                from = point_from_param1(&curve);
            }
            svg::node::element::path::Command::Line(pos, params) => {
                for raw_curve in &params.iter().chunks(2) {
                    let curve: Vec<&f32> = raw_curve.collect();

                    let to = point_for_position(pos, Pt::new(*curve[0], *curve[1]), from);

                    let line = lyon::geom::LineSegment {
                        from: from.into(),
                        to: to.into(),
                    };

                    let length = line.length();

                    segment_state.add_segment(line, length);

                    from = to;
                }
            }
            svg::node::element::path::Command::HorizontalLine(pos, params) => {
                for next_point in params.iter() {
                    let to = match pos {
                        svg::node::element::path::Position::Absolute => {
                            Pt::new(*next_point, from.y())
                        }
                        svg::node::element::path::Position::Relative => {
                            Pt::new(next_point + from.x(), from.y())
                        }
                    };

                    let line = lyon::geom::LineSegment {
                        from: from.into(),
                        to: to.into(),
                    };

                    let length = line.length();

                    segment_state.add_segment(line, length);

                    from = to;
                }
            }
            svg::node::element::path::Command::VerticalLine(pos, params) => {
                for next_point in params.iter() {
                    let to = match pos {
                        svg::node::element::path::Position::Absolute => {
                            Pt::new(from.x(), *next_point)
                        }
                        svg::node::element::path::Position::Relative => {
                            Pt::new(from.x(), next_point + from.y())
                        }
                    };

                    let line = lyon::geom::LineSegment {
                        from: from.into(),
                        to: to.into(),
                    };

                    let length = line.length();

                    segment_state.add_segment(line, length);

                    from = to;
                }
            }
            svg::node::element::path::Command::CubicCurve(pos, params) => {
                for raw_curve in &params.iter().chunks(6) {
                    let curve: Vec<&f32> = raw_curve.collect();
                    let (raw_ctrl1, raw_ctrl2, raw_to) = point_from_param3(&curve);

                    let ctrl1 = point_for_position(pos, raw_ctrl1, from);
                    let ctrl2 = point_for_position(pos, raw_ctrl2, from);
                    let to = point_for_position(pos, raw_to, from);

                    let line = lyon::geom::CubicBezierSegment {
                        from: from.into(),
                        ctrl1: ctrl1.into(),
                        ctrl2: ctrl2.into(),
                        to: to.into(),
                    };

                    let length = line.approximate_length(0.1);

                    segment_state.add_segment(line, length);

                    // prev_ctrl = Some(raw_ctrl2);

                    from = to;
                }
            }
            svg::node::element::path::Command::SmoothCubicCurve(
                svg::node::element::path::Position::Relative,
                params,
            ) => {
                for raw_curve in &params.iter().chunks(4) {
                    let curve: Vec<&f32> = raw_curve.collect();
                    let (raw_ctrl2, raw_to) = point_from_param2(&curve);

                    let ctrl2 = raw_ctrl2 + from;
                    let to = raw_to + from;

                    let ctrl1 = Pt::new(from.x(), from.y()); // i'm.. surprised this works

                    let line = lyon::geom::CubicBezierSegment {
                        from: from.into(),
                        ctrl1: ctrl1.into(),
                        ctrl2: ctrl2.into(),
                        to: to.into(),
                    };

                    let length = line.approximate_length(0.1);

                    segment_state.add_segment(line, length);

                    from = to;
                }
            }
            svg::node::element::path::Command::Close => {}
            svg::node::element::path::Command::QuadraticCurve(pos, params) => {
                for raw_curve in &params.iter().chunks(4) {
                    let curve: Vec<&f32> = raw_curve.collect();
                    let (raw_ctrl, raw_to) = point_from_param2(&curve);

                    let to = point_for_position(pos, raw_to, from);
                    let ctrl = point_for_position(pos, raw_ctrl, from);

                    let line = lyon::geom::QuadraticBezierSegment {
                        from: from.into(),
                        ctrl: ctrl.into(),
                        to: to.into(),
                    };

                    let length = line.approximate_length(0.1);

                    segment_state.add_segment(line, length);

                    from = to;
                }
            }
            svg::node::element::path::Command::SmoothQuadraticCurve(_, _) => todo!(),
            svg::node::element::path::Command::EllipticalArc(_, _) => todo!(),
            _ => todo!(),
        };
    }

    // println!("processed {:?} pts", segment_state.vertices.len());

    segment_state
        .vertices
        .into_iter()
        .map(|x| x.as_vec2())
        .collect_vec()
}

#[derive(Debug, Copy, Clone)]
pub struct Pt {
    pt: Point2D<f32, UnknownUnit>,
}
impl Pt {
    pub fn new(x: f32, y: f32) -> Pt {
        Pt {
            pt: Point2D::<f32, UnknownUnit>::new(x, y),
        }
    }

    fn x(&self) -> f32 {
        self.pt.x
    }

    fn y(&self) -> f32 {
        self.pt.y
    }

    pub fn as_vec2(&self) -> Vec2 {
        Vec2::new(self.y(), self.x())
    }
}

impl ops::Add<Pt> for Pt {
    type Output = Pt;

    fn add(self, rhs: Pt) -> Pt {
        Pt::new(self.x() + rhs.x(), self.y() + rhs.y())
    }
}

impl From<Pt> for Point2D<f32, UnknownUnit> {
    fn from(val: Pt) -> Self {
        val.pt
    }
}

fn point_for_position(pos: &svg::node::element::path::Position, pt: Pt, from: Pt) -> Pt {
    match pos {
        svg::node::element::path::Position::Absolute => pt,
        svg::node::element::path::Position::Relative => pt + from,
    }
}

pub struct SegmentState {
    vertices: Vec<Pt>,
    line_space: f32,
    dist_towards_next: f32,
}
impl Default for SegmentState {
    fn default() -> Self {
        Self::new()
    }
}

impl SegmentState {
    pub fn new() -> SegmentState {
        SegmentState {
            vertices: Vec::<Pt>::new(),
            line_space: 5.0,
            dist_towards_next: 0.0,
        }
    }

    pub fn new_with_line_space(line_space: f32) -> SegmentState {
        SegmentState {
            vertices: Vec::<Pt>::new(),
            line_space,
            dist_towards_next: 0.0,
        }
    }

    fn update(&mut self, length: f32, new_vertices: Vec<Pt>) {
        self.dist_towards_next = (length + self.dist_towards_next) % self.line_space;
        self.vertices.extend(new_vertices);
    }
    pub fn vertices(&self) -> Vec<Vec2> {
        self.vertices.iter().map(|x| vec2(x.x(), x.y())).collect()
    }

    pub fn add_segment(&mut self, segment: impl lyon::geom::Segment<Scalar = f32>, length: f32) {
        let mut vertices: Vec<Pt> = Vec::<Pt>::new();

        let pt_count = ((length) / self.line_space) as u32;

        // println!("pt count {:?}", pt_count);
        // println!("{:?}", self.dist_towards_next);

        // if it's an even number, we'll need one more. just include it, then
        // trim it when t turns out > 1
        for pt_i in 0..=pt_count {
            let t_n = (self.line_space * pt_i as f32) + self.dist_towards_next;
            let t = t_n / length;
            // println!("{:?} {:?}", t_n, t);

            if t <= 1.0 {
                let x = segment.x(t);
                let y = segment.y(t);
                // println!("({:?}, {:?})", x, y);
                vertices.push(Pt::new(x, y));
            }
        }

        self.update(length, vertices)
    }
}

pub fn load_all_data<T>(path: T, line_space: f32) -> HashMap<String, Vec<Vec<Vec2>>>
where
    T: AsRef<std::path::Path>,
{
    let map = load_all_data_into_map(path);

    let r: HashMap<String, Vec<Vec<Vec2>>> = map
        .iter()
        .map(|(k, v)| {
            // println!("processing {:?}", k);
            (
                k.to_string(),
                v.iter().map(|vv| parse_data(vv, line_space)).collect_vec(),
            )
        })
        .collect();
    r
}

pub fn load_all_data_into_map<T>(path: T) -> HashMap<String, Vec<svg::node::element::path::Data>>
where
    T: AsRef<std::path::Path>,
{
    let mut content = String::new();

    let mut maps: HashMap<String, Vec<svg::node::element::path::Data>> = HashMap::new();

    let mut recent_id: String = "".to_string(); // i hate this

    for event in svg::open(path, &mut content).unwrap() {
        if let svg::parser::Event::Tag(_, _, attributes) = event {
            if let Some(id) = attributes.get("id") {
                recent_id = id.to_string();
            }

            if let Some(path_data) = attributes.get("d") {
                let data = svg::node::element::path::Data::parse(path_data).unwrap();
                maps.entry(recent_id.to_owned()).or_default().push(data);
            }
        };
    }

    maps
}

// SvgPathDef is a simplified svg thingy.. this just converts back to the full
// svg and then parses like usual
pub fn parse_svg_path_as_vec2(data: &SvgPathDef, line_space: f32) -> Vec<Vec2> {
    let mut cmds = svg::node::element::path::Data::new();
    let (start_x, start_y) = data.svg_move_to();
    cmds = cmds.move_to(vec![start_x, start_y]);

    for cmd in data.cmds() {
        match cmd {
            crate::svg::SvgCmd::Line(svg_to) => {
                let (x, y) = svg_to.params();
                cmds = cmds.line_to(vec![x, y]);
            }
            crate::svg::SvgCmd::CubicBezier(svg_cubic_bezier) => {
                let (a, b, c, d, e, f) = svg_cubic_bezier.params();
                cmds = cmds.cubic_curve_to(vec![a, b, c, d, e, f]);
            }
            crate::svg::SvgCmd::ArcTo(svg_arc) => {
                let (a, b, c, d, e, f, g) = svg_arc.params();
                cmds = cmds.elliptical_arc_to(vec![a, b, c, d, e, f, g]);
            }
            crate::svg::SvgCmd::Close => {
                cmds = cmds.close();
            }
        }
    }

    parse_svg_data_as_vec2(&cmds, line_space)
}

// todo, can i combine this with the output?
pub struct LayersFromSvg {
    pub layers: HashMap<String, Vec<Polyline>>,
}
impl LayersFromSvg {
    pub fn load<T>(path: T) -> LayersFromSvg
    where
        T: AsRef<std::path::Path>,
    {
        let vecs = load_all_data(path, 5.0);

        let mut layers = HashMap::new();
        for (layer_name, vec) in &vecs {
            let polylines = vec.iter().map(|x| Polyline::new(x.clone())).collect();
            layers.insert(layer_name.clone(), polylines);
        }

        LayersFromSvg { layers }
    }
}
