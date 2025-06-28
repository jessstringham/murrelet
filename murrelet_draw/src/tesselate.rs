use std::{collections::HashMap, ops};

use crate::{
    cubic::CubicBezier,
    // curve_drawer::{CurveDrawer, CurveSegment},
    svg::SvgPathDef,
};
use glam::{vec2, Vec2, Vec2Swizzles};
use itertools::Itertools;
use kurbo::BezPath;
use lyon::{
    geom::{
        euclid::{Point2D, UnknownUnit},
        point,
    },
    path::{FillRule, Path},
    tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex},
};
use murrelet_common::{triangulate::VertexSimple, Polyline};

pub trait ToVecVec2 {
    fn to_vec2(&self) -> Vec<Vec2>;
}

impl ToVecVec2 for CubicBezier {
    fn to_vec2(&self) -> Vec<Vec2> {
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

        let mut path = parse_svg_data_as_vec2(&svg, 1.0);

        if let Some(a) = path.last() {
            if a.distance(self.to.yx()) > 1.0e-3 {
                path.push(self.to.yx())
            }
        }

        path.into_iter().map(|x| vec2(x.y, x.x)).collect_vec()
    }
}

fn vec2_to_kurbo(v: Vec2) -> kurbo::Point {
    kurbo::Point::new(v.x as f64, v.y as f64)
}

fn vec2_to_pt(x: Vec2) -> lyon::geom::euclid::Point2D<f32, lyon::geom::euclid::UnknownUnit> {
    point(x.x, x.y)
}

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

// fn point_from_param4(params: &Vec<&f32>) -> (Pt, Pt, Pt, Pt) {
//     (
//         _point_from_params(params, 0),
//         _point_from_params(params, 1),
//         _point_from_params(params, 2),
//         _point_from_params(params, 3),
//     )
// }

pub fn many_pt2_to_vec2(ps: &Vec<Pt>) -> Vec<Vec2> {
    ps.iter().map(|p| p.as_vec2()).collect_vec()
}

pub fn cubic_bezier_path_to_lyon(path: &[CubicBezier], closed: bool) -> Option<lyon::path::Path> {
    // let mut builder = Path::builder();

    if path.is_empty() {
        return None;
    }
    let mut kurbo_path = BezPath::new();

    kurbo_path.move_to(vec2_to_kurbo(path[0].from));
    for c in path {
        kurbo_path.curve_to(
            vec2_to_kurbo(c.ctrl1),
            vec2_to_kurbo(c.ctrl2),
            vec2_to_kurbo(c.to),
        )
    }

    if closed {
        kurbo_path.close_path();
    }

    let tolerance = 0.01;

    let mut lyon_builder = lyon::path::Path::builder();
    kurbo::flatten(kurbo_path, tolerance, |el| {
        match el {
            kurbo::PathEl::MoveTo(p) => {
                lyon_builder.begin(point(p.x as f32, p.y as f32));
            }
            kurbo::PathEl::LineTo(p) => {
                lyon_builder.line_to(point(p.x as f32, p.y as f32));
            }
            kurbo::PathEl::ClosePath => lyon_builder.close(),
            // The flatten iterator produces only MoveTo, LineTo, and ClosePath.
            _ => {}
        }
    });
    let path = lyon_builder.build();
    Some(path)
}

pub fn tesselate_lyon_vertex_simple(outline: &[VertexSimple]) -> (Vec<u32>, Vec<VertexSimple>) {
    let mut path_builder = Path::builder_with_attributes(5);

    // convert path to lyon
    if let Some(first_vertex) = outline.first() {
        path_builder.begin(vec2_to_pt(first_vertex.pos2d()), &first_vertex.attrs());
        for vertex in outline.iter().skip(1) {
            path_builder.line_to(vec2_to_pt(vertex.pos2d()), &vertex.attrs());
        }
        path_builder.close();
    } else {
        return (Vec::new(), Vec::new());
    }

    let path = path_builder.build();

    let opts = FillOptions::default()
        .with_tolerance(10.1)
        .with_fill_rule(FillRule::EvenOdd)
        .with_intersections(true);

    let mut geometry: lyon::lyon_tessellation::VertexBuffers<VertexSimple, u32> =
        lyon::lyon_tessellation::VertexBuffers::new();
    let mut tess = FillTessellator::new();
    tess.tessellate_path(
        path.as_slice(),
        &opts,
        &mut BuffersBuilder::new(&mut geometry, |mut v: FillVertex| {
            let pos = v.position();
            let attrs = v.interpolated_attributes();

            VertexSimple {
                position: [pos.x, pos.y, 0.0],
                normal: [attrs[0], attrs[1], attrs[2]],
                face_pos: [attrs[3], attrs[4]],
            }
        }),
    )
    .expect("tessellation failed");

    (geometry.indices, geometry.vertices)
}

pub fn tesselate_lyon(path: &Path) -> (Vec<u32>, Vec<[f32; 3]>) {
    let opts = FillOptions::default()
        .with_tolerance(0.1)
        .with_fill_rule(FillRule::EvenOdd)
        .with_intersections(true);

    let mut geometry: lyon::lyon_tessellation::VertexBuffers<[f32; 3], u32> =
        lyon::lyon_tessellation::VertexBuffers::new();
    let mut tess = FillTessellator::new();
    tess.tessellate_path(
        path.as_slice(),
        &opts,
        &mut BuffersBuilder::new(&mut geometry, |v: FillVertex| {
            let p = v.position();
            [p.x, p.y, 0.0]
        }),
    )
    .expect("tessellation failed");

    (geometry.indices, geometry.vertices)
}

pub fn parse_svg_data_as_vec2(data: &svg::node::element::path::Data, tolerance: f32) -> Vec<Vec2> {
    parse_data(data, tolerance)
}

// svg loader
fn parse_data(data: &svg::node::element::path::Data, tolerance: f32) -> Vec<Vec2> {
    let mut segment_state = SegmentState::new_with_line_space(tolerance);

    let mut from = Pt::new(0.0, 0.0);

    // this is needed for smooth cubic blah
    // let mut prev_ctrl = None;

    // https://developer.mozilla.org/en-US/docs/Web/SVG/Tutorial/Paths
    for command in data.iter() {
        // println!("{:?}", command);

        let _ly = match command {
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

                    // let ctrl1_raw = prev_ctrl.unwrap(); // better exist
                    // let ctrl1 = point(from.x - ctrl1_raw.y, from.y - ctrl1_raw.x);
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

impl Into<Point2D<f32, UnknownUnit>> for Pt {
    fn into(self) -> Point2D<f32, UnknownUnit> {
        self.pt
    }
}

fn point_for_position(pos: &svg::node::element::path::Position, pt: Pt, from: Pt) -> Pt {
    match pos {
        svg::node::element::path::Position::Absolute => pt.into(),
        svg::node::element::path::Position::Relative => (pt + from).into(),
    }
}

pub struct SegmentState {
    vertices: Vec<Pt>,
    line_space: f32,
    dist_towards_next: f32,
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

// fn parse_data_as_curve(data: &svg::node::element::path::Data, _tolerance: f32) -> CurveDrawer {
//     let mut curve_segments: Vec<CurveSegment> = vec![];

//     let mut from = Pt::new(0.0, 0.0);
//     let mut close = false;

//     // https://developer.mozilla.org/en-US/docs/Web/SVG/Tutorial/Paths
//     for command in data.iter() {
//         // println!("{:?}", command);

//         let _ly = match command {
//             svg::node::element::path::Command::Move(_pos, params) => {
//                 let curve: Vec<&f32> = params.iter().collect();
//                 from = point_from_param1(&curve);

//                 curve_segments.push(CurveSegment::new_simple_point(from.as_vec2()));
//             }
//             svg::node::element::path::Command::Line(pos, params) => {
//                 for raw_curve in &params.iter().chunks(2) {
//                     let curve: Vec<&f32> = raw_curve.collect();

//                     let to = point_for_position(pos, Pt::new(*curve[0], *curve[1]), from);

//                     // let line = lyon::geom::LineSegment {
//                     //     from: from.into(),
//                     //     to: to.into(),
//                     // };

//                     // let length = line.length();

//                     // segment_state.add_segment(line, length);
//                     curve_segments.push(CurveSegment::new_simple_point(to.as_vec2()));

//                     from = to;
//                 }
//             }
//             svg::node::element::path::Command::HorizontalLine(_, _) => todo!(),
//             svg::node::element::path::Command::VerticalLine(
//                 svg::node::element::path::Position::Relative,
//                 params,
//             ) => {
//                 for next_point in params.iter() {
//                     let to = Pt::new(from.x(), next_point + from.y());

//                     // let line = lyon::geom::LineSegment {
//                     //     from: from.into(),
//                     //     to: to.into(),
//                     // };

//                     // let length = line.length();

//                     // segment_state.add_segment(line, length);

//                     curve_segments.push(CurveSegment::new_simple_point(to.as_vec2()));

//                     from = to;
//                 }
//             }
//             svg::node::element::path::Command::CubicCurve(pos, params) => {
//                 for raw_curve in &params.iter().chunks(6) {
//                     let curve: Vec<&f32> = raw_curve.collect();
//                     let (raw_ctrl1, raw_ctrl2, raw_to) = point_from_param3(&curve);

//                     let _ctrl1 = point_for_position(pos, raw_ctrl1, from);
//                     let _ctrl2 = point_for_position(pos, raw_ctrl2, from);
//                     let _to = point_for_position(pos, raw_to, from);

//                     // let line = lyon::geom::CubicBezierSegment {
//                     //     from: from.into(),
//                     //     ctrl1: ctrl1.into(),
//                     //     ctrl2: ctrl2.into(),
//                     //     to: to.into(),
//                     // };

//                     // let length = line.approximate_length(0.1);

//                     // segment_state.add_segment(line, length);

//                     todo!("cubic!");
//                     // curve_segments.push(CurveSegment::new_simple_point(to.as_vec2()));

//                     // prev_ctrl = Some(raw_ctrl2);

//                     //from = to;
//                 }
//             }
//             svg::node::element::path::Command::SmoothCubicCurve(
//                 svg::node::element::path::Position::Relative,
//                 params,
//             ) => {
//                 for raw_curve in &params.iter().chunks(4) {
//                     let curve: Vec<&f32> = raw_curve.collect();
//                     let (raw_ctrl2, raw_to) = point_from_param2(&curve);

//                     let ctrl2 = raw_ctrl2 + from;
//                     let to = raw_to + from;

//                     // let ctrl1_raw = prev_ctrl.unwrap(); // better exist
//                     // let ctrl1 = point(from.x - ctrl1_raw.y, from.y - ctrl1_raw.x);
//                     let ctrl1 = Pt::new(from.x(), from.y()); // i'm.. surprised this works

//                     let _line = lyon::geom::CubicBezierSegment {
//                         from: from.into(),
//                         ctrl1: ctrl1.into(),
//                         ctrl2: ctrl2.into(),
//                         to: to.into(),
//                     };

//                     // let length = line.approximate_length(0.1);
//                     // segment_state.add_segment(line, length);
//                     todo!("cubic!");

//                     //from = to;
//                 }
//             }
//             svg::node::element::path::Command::Close => {
//                 close = true;
//             }
//             svg::node::element::path::Command::QuadraticCurve(pos, params) => {
//                 for raw_curve in &params.iter().chunks(4) {
//                     let curve: Vec<&f32> = raw_curve.collect();
//                     let (raw_ctrl, raw_to) = point_from_param2(&curve);

//                     let to = point_for_position(pos, raw_to, from);
//                     let ctrl = point_for_position(pos, raw_ctrl, from);

//                     let _line = lyon::geom::QuadraticBezierSegment {
//                         from: from.into(),
//                         ctrl: ctrl.into(),
//                         to: to.into(),
//                     };

//                     todo!("quad!");

//                     // let length = line.approximate_length(0.1);

//                     // segment_state.add_segment(line, length);

//                     //from = to;
//                 }
//             }
//             svg::node::element::path::Command::SmoothQuadraticCurve(_, _) => todo!(),
//             svg::node::element::path::Command::EllipticalArc(_, _) => todo!(),
//             _ => todo!(),
//         };
//     }

//     // println!("processed {:?} pts", segment_state.vertices.len());

//     // segment_state.vertices

//     // todo, figure out closed

//     CurveDrawer::new(curve_segments, close)
// }

pub fn load_all_data<T>(path: T, tolerance: f32) -> HashMap<String, Vec<Vec<Vec2>>>
where
    T: AsRef<std::path::Path>,
{
    let map = load_all_data_into_map(path);

    // println!("loaded into map");

    let r: HashMap<String, Vec<Vec<Vec2>>> = map
        .iter()
        .map(|(k, v)| {
            println!("processing {:?}", k);
            (
                k.to_string(),
                v.iter().map(|vv| parse_data(vv, tolerance)).collect_vec(),
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
                println!("loading {:?}", id);
                recent_id = id.to_string();
            }

            if let Some(path_data) = attributes.get("d") {
                // println!("path_data {:?}", path_data);
                let data = svg::node::element::path::Data::parse(path_data).unwrap();
                maps.entry(recent_id.to_owned()).or_default().push(data);
            }
        };
    }

    maps
}

// SvgPathDef is a simplified svg thingy.. this just converts back to the full
// svg and then parses like usual
pub fn parse_svg_path_as_vec2(data: &SvgPathDef, tolerance: f32) -> Vec<Vec2> {
    let mut cmds = svg::node::element::path::Data::new();
    let (start_x, start_y) = data.svg_move_to();
    cmds = cmds.move_to(vec![start_x, start_y]);
    // (Command::Move(Position::Absolute, );

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
        }
    }

    parse_svg_data_as_vec2(&cmds, tolerance)
}

// fn point_to_param(pt: &Point2D<f32, UnknownUnit>) -> Vec<f32> {
//     vec![pt.x, pt.y]
// }

// fn points_to_param(pts: Vec<&Point2D<f32, UnknownUnit>>) -> Vec<f32> {
//     pts.iter().map(|pt| point_to_param(*pt)).flatten().collect()
// }
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
