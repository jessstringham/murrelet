use std::f32::consts::PI;

use crate::{
    convert::{glam_to_kurbo, glam_to_lyon},
    cubic::CubicBezier,
    curve_drawer::{CurveArc, CurveCubicBezier, CurveDrawer, CurvePoints, CurveSegment},
};
use delaunator::Triangulation;
use glam::{Vec2, vec2};
use itertools::Itertools;
use kurbo::BezPath;
use lyon::{
    geom::Angle,
    path::{Event, traits::Build},
};
use lyon::{geom::arc::Arc, math::Transform};
use lyon::{geom::vector, path::traits::PathIterator};
use lyon::{
    geom::{Point, point},
    path::{FillRule, Path, traits::PathBuilder},
    tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex},
};
use murrelet_common::{
    IsAngle, SimpleTransform2d, SimpleTransform2dStep, ToVec2, triangulate::DefaultVertex,
};
use murrelet_livecode::types::{LivecodeError, LivecodeResult};

pub trait AsLyonTransform {
    fn to_lyon_transform(&self) -> Transform {
        self.update_lyon_transform(Transform::identity())
    }

    fn update_lyon_transform(&self, t: Transform) -> Transform;
}

impl AsLyonTransform for SimpleTransform2d {
    fn update_lyon_transform(&self, t: Transform) -> Transform {
        let mut aa = t;
        for t in self.steps() {
            aa = t.update_lyon_transform(aa);
        }
        aa
    }
}

impl AsLyonTransform for SimpleTransform2dStep {
    fn update_lyon_transform(&self, t: Transform) -> Transform {
        match self {
            SimpleTransform2dStep::Translate(v) => t.then_translate(vector(v.x, v.y)),
            SimpleTransform2dStep::Rotate(v, a) => {
                let vv = vector(v.x, v.y);
                let t = t.then_translate(-vv);
                let t = t.then_rotate(Angle::radians(a.angle()));

                t.then_translate(vv)
            }
            SimpleTransform2dStep::Scale(v) => t.then_scale(v.x, v.y),
            SimpleTransform2dStep::Skew(_, _) => unreachable!(),
        }
    }
}

pub trait ToLyonPath {
    const EPS: f32 = 1e-6f32;

    fn approx_vertex_count(&self) -> usize;

    fn to_lyon_with_transform<T: AsLyonTransform>(
        &self,
        t: &T,
    ) -> LivecodeResult<lyon::path::Path> {
        let transform = t.to_lyon_transform();

        let mut lyon_builder = lyon::path::Path::builder().transformed(transform);

        let start = self.start();

        lyon_builder.begin(glam_to_lyon(start));
        let is_closed = self._add_to_lyon(start, &mut lyon_builder)?;
        lyon_builder.end(is_closed);

        Ok(lyon_builder.build())
    }

    fn to_lyon(&self) -> LivecodeResult<lyon::path::Path> {
        self.to_lyon_with_transform(&SimpleTransform2d::ident())
    }

    fn flatten_with_lyon(&self, tolerance: f32) -> LivecodeResult<Vec<Vec2>> {
        let path = self.to_lyon()?;

        let mut pts = vec![];

        for evt in path.iter().flattened(tolerance) {
            match evt {
                Event::Begin { at } => {
                    let p = Vec2::new(at.x, at.y);
                    pts.push(p);
                }
                Event::Line { to, .. } => {
                    pts.push(Vec2::new(to.x, to.y));
                }
                Event::End {
                    last,
                    first: f,
                    close,
                } => {
                    if close {
                        let a = Vec2::new(last.x, last.y);
                        let b = Vec2::new(f.x, f.y);
                        if a != b {
                            pts.push(b); // close loop
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(pts)
    }

    fn start(&self) -> Vec2;

    fn _start(&self) -> Point<f32> {
        glam_to_lyon(self.start())
    }

    fn _add_to_lyon<B: PathBuilder>(&self, start: Vec2, builder: &mut B) -> LivecodeResult<bool> {
        // handles when "start" is too far away
        if start.distance(self.start()) > Self::EPS {
            builder.line_to(self._start());
        }

        self.add_to_lyon(builder)
    }

    fn add_to_lyon<B: PathBuilder>(&self, builder: &mut B) -> LivecodeResult<bool>;
}

impl ToLyonPath for CurvePoints {
    fn start(&self) -> Vec2 {
        self.first_point()
    }

    fn add_to_lyon<B: PathBuilder>(&self, builder: &mut B) -> LivecodeResult<bool> {
        for p in self.points() {
            if p.x.is_nan() || p.y.is_nan() {
                return LivecodeError::rawr("nan in CurvePoints");
            }

            builder.line_to(glam_to_lyon(*p));
        }

        Ok(false)
    }

    fn approx_vertex_count(&self) -> usize {
        self.points.len()
    }
}

impl ToLyonPath for CubicBezier {
    fn start(&self) -> Vec2 {
        self.from
    }

    fn add_to_lyon<B: PathBuilder>(&self, builder: &mut B) -> LivecodeResult<bool> {
        if self.ctrl1.is_nan() || self.ctrl2.is_nan() || self.to.is_nan() || self.from.is_nan() {
            return LivecodeError::rawr("nan in Bezier");
        }

        builder.cubic_bezier_to(glam_to_lyon(self.ctrl1), glam_to_lyon(self.ctrl2), glam_to_lyon(self.to));

        Ok(false)
    }

    fn approx_vertex_count(&self) -> usize {
        4
    }
}

impl ToLyonPath for CurveCubicBezier {
    fn start(&self) -> Vec2 {
        self.to_cubic().start()
    }

    fn add_to_lyon<B: PathBuilder>(&self, builder: &mut B) -> LivecodeResult<bool> {
        self.to_cubic().add_to_lyon(builder)?;

        Ok(false)
    }

    fn approx_vertex_count(&self) -> usize {
        self.to_cubic().approx_vertex_count()
    }
}

// chatgpt
fn add_circular_arc<B: PathBuilder>(builder: &mut B, c: &CurveArc) {
    let cx = c.loc.x;
    let cy = c.loc.y;
    let r = c.radius;
    let start = c.start_pi().angle();
    let end: f32 = c.end_pi().angle();

    // Angles are in radians.
    let mut sweep = end - start;

    if c.is_ccw() {
        if sweep < 0.0 {
            sweep += 2.0 * PI;
        }
    } else if sweep > 0.0 {
        sweep -= 2.0 * PI;
    }

    let arc = Arc {
        center: point(cx, cy),
        radii: vector(r, r),
        start_angle: Angle::radians(start),
        sweep_angle: Angle::radians(sweep),
        x_rotation: Angle::radians(0.0),
    };

    arc.for_each_cubic_bezier(&mut |c| {
        builder.cubic_bezier_to(c.ctrl1, c.ctrl2, c.to);
    });
}

impl ToLyonPath for CurveArc {
    fn start(&self) -> Vec2 {
        self.first_point()
    }

    fn add_to_lyon<B: PathBuilder>(&self, builder: &mut B) -> LivecodeResult<bool> {
        if self.end_pi.angle_pi().is_nan()
            || self.start_pi.angle_pi().is_nan()
            || self.radius.is_nan()
            || self.loc.x.is_nan()
            || self.loc.y.is_nan()
        {
            return LivecodeError::rawr("nan in CurveArc");
        }

        add_circular_arc(builder, self);

        Ok(false)
    }

    fn approx_vertex_count(&self) -> usize {
        4
    }
}

impl ToLyonPath for CurveSegment {
    fn start(&self) -> Vec2 {
        self.first_point()
    }

    fn add_to_lyon<B: PathBuilder>(&self, builder: &mut B) -> LivecodeResult<bool> {
        match self {
            CurveSegment::Arc(c) => c.add_to_lyon(builder),
            CurveSegment::Points(c) => c.add_to_lyon(builder),
            CurveSegment::CubicBezier(c) => c.add_to_lyon(builder),
        }
    }

    fn approx_vertex_count(&self) -> usize {
        match self {
            CurveSegment::Arc(c) => c.approx_vertex_count(),
            CurveSegment::Points(c) => c.approx_vertex_count(),
            CurveSegment::CubicBezier(c) => c.approx_vertex_count(),
        }
    }
}

impl ToLyonPath for CurveDrawer {
    fn start(&self) -> Vec2 {
        self.first_point().unwrap_or_default() //???
    }

    fn add_to_lyon<B: PathBuilder>(&self, builder: &mut B) -> LivecodeResult<bool> {
        for s in self.segments() {
            s.add_to_lyon(builder)?;
        }
        Ok(self.closed)
    }

    fn approx_vertex_count(&self) -> usize {
        let mut c = 0;
        for s in self.segments() {
            c += s.approx_vertex_count();
        }
        c
    }
}

pub fn cubic_bezier_path_to_lyon(path: &[CubicBezier], closed: bool) -> Option<lyon::path::Path> {
    // let mut builder = Path::builder();

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

pub fn tesselate_lyon_vertex_with_steiner(
    outline: &[DefaultVertex],
    steiner: &[DefaultVertex],
) -> (Vec<u32>, Vec<DefaultVertex>) {
    let mut path_builder = Path::builder_with_attributes(5);

    // convert path to lyon
    if let Some(first_vertex) = outline.first() {
        path_builder.begin(glam_to_lyon(first_vertex.pos2d()), &first_vertex.attrs());
        for vertex in outline.iter().skip(1) {
            path_builder.line_to(glam_to_lyon(vertex.pos2d()), &vertex.attrs());
        }
        path_builder.close();
    } else {
        return (Vec::new(), Vec::new());
    }

    let amount = 1e-6f32;
    for s in steiner {
        // now poke holes in it
        let loc = s.pos2d();

        let p0 = loc + vec2(amount, 0.0);
        let p1 = loc + vec2(0.0, amount);
        let p2 = loc + vec2(-amount, 0.0);
        path_builder.begin(glam_to_lyon(p0), &s.attrs());
        path_builder.line_to(glam_to_lyon(p1), &s.attrs());
        path_builder.line_to(glam_to_lyon(p2), &s.attrs());
        path_builder.close()
    }

    let path = path_builder.build();

    let opts = FillOptions::default()
        .with_fill_rule(FillRule::EvenOdd)
        .with_intersections(true);

    let mut geometry: lyon::lyon_tessellation::VertexBuffers<DefaultVertex, u32> =
        lyon::lyon_tessellation::VertexBuffers::new();
    let mut tess = FillTessellator::new();
    tess.tessellate_path(
        path.as_slice(),
        &opts,
        &mut BuffersBuilder::new(&mut geometry, |mut v: FillVertex| {
            let pos = v.position();
            let attrs = v.interpolated_attributes();

            DefaultVertex {
                position: [pos.x, pos.y, 0.0],
                normal: [attrs[0], attrs[1], attrs[2]],
                face_pos: [attrs[3], attrs[4]],
            }
        }),
    )
    .expect("tessellation failed");

    (geometry.indices, geometry.vertices)
}

pub fn tesselate_lyon_vertex_simple(outline: &[DefaultVertex]) -> (Vec<u32>, Vec<DefaultVertex>) {
    let mut path_builder = Path::builder_with_attributes(5);

    // convert path to lyon
    if let Some(first_vertex) = outline.first() {
        path_builder.begin(glam_to_lyon(first_vertex.pos2d()), &first_vertex.attrs());
        for vertex in outline.iter().skip(1) {
            path_builder.line_to(glam_to_lyon(vertex.pos2d()), &vertex.attrs());
        }
        path_builder.close();
    } else {
        return (Vec::new(), Vec::new());
    }

    let path = path_builder.build();

    let opts = FillOptions::default()
        .with_fill_rule(FillRule::EvenOdd)
        .with_intersections(true);

    let mut geometry: lyon::lyon_tessellation::VertexBuffers<DefaultVertex, u32> =
        lyon::lyon_tessellation::VertexBuffers::new();
    let mut tess = FillTessellator::new();
    tess.tessellate_path(
        path.as_slice(),
        &opts,
        &mut BuffersBuilder::new(&mut geometry, |mut v: FillVertex| {
            let pos = v.position();
            let attrs = v.interpolated_attributes();

            DefaultVertex {
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

pub fn tesselate_delauney<VertexKind: ToVec2 + Clone>(
    v: Vec<VertexKind>,
) -> (Vec<u32>, Vec<VertexKind>, Triangulation) {
    let points: Vec<_> = v
        .iter()
        .map(|vertex| {
            let loc = vertex.to_vec2();
            delaunator::Point {
                x: loc.x as f64,
                y: loc.y as f64,
            }
        })
        .collect();
    let triangulation = delaunator::triangulate(&points);

    // chatgpt
    fn point_in_poly(x: f64, y: f64, poly: &[(f64, f64)]) -> bool {
        let mut inside = false;
        let n = poly.len();
        for i in 0..n {
            let (xi, yi) = poly[i];
            let (xj, yj) = poly[(i + 1) % n];
            let intersect = ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
            if intersect {
                inside = !inside;
            }
        }
        inside
    }

    let mut filtered_indices = Vec::new();
    for tri in triangulation.triangles.chunks(3) {
        let a = &points[tri[0]];
        let b = &points[tri[1]];
        let c = &points[tri[2]];
        let cx = (a.x + b.x + c.x) / 3.0;
        let cy = (a.y + b.y + c.y) / 3.0;
        if point_in_poly(
            cx,
            cy,
            &v.iter()
                .map(|p| {
                    let loc = p.to_vec2();
                    (loc.x as f64, loc.y as f64)
                })
                .collect::<Vec<_>>(),
        ) {
            filtered_indices.extend_from_slice(&[tri[0] as u32, tri[1] as u32, tri[2] as u32]);
        }
    }

    let vertices = v.clone();
    (filtered_indices, vertices, triangulation)
}

pub fn tesselate_delauney_no_filter<VertexKind: ToVec2 + Clone>(
    v: Vec<VertexKind>,
) -> (Vec<u32>, Vec<VertexKind>, Triangulation) {
    let points: Vec<_> = v
        .iter()
        .map(|vertex| {
            let loc = vertex.to_vec2();
            delaunator::Point {
                x: loc.x as f64,
                y: loc.y as f64,
            }
        })
        .collect();
    let triangulation = delaunator::triangulate(&points);

    let indices = triangulation
        .triangles
        .iter()
        .map(|x| *x as u32)
        .collect_vec();
    (indices, v, triangulation)
}
