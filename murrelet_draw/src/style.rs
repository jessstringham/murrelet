#![allow(dead_code)]
use std::collections::HashMap;

use crate::{curve_drawer::CurveDrawer, draw::*, svg::TransformedSvgShape, transform2d::*};
use glam::*;
use lerpable::Lerpable;
use md5::{Digest, Md5};
use murrelet_common::*;
use murrelet_gui::{CanMakeGUI, MurreletGUI};
use murrelet_livecode::{lazy::ControlLazyNodeF32, livecode::ControlF32, types::ControlVecElement};
use murrelet_livecode_derive::Livecode;
use styleconf::StyleConf;

fn _black() -> [ControlF32; 4] {
    [
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(1.0),
    ]
}

fn _black_lazy() -> Vec<ControlVecElement<ControlLazyNodeF32>> {
    vec![
        ControlVecElement::Single(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::Single(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::Single(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::Single(ControlLazyNodeF32::Float(1.0)),
    ]
}

#[derive(Copy, Clone, Debug, Livecode, Lerpable, Default)]
pub struct MurreletStyleFilled {
    pub color: MurreletColor, // fill color
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
    #[livecode(serde_default = "_black")]
    pub stroke_color: MurreletColor,
}
impl MurreletStyleFilled {
    pub fn new(color: MurreletColor, stroke_weight: f32, stroke_color: MurreletColor) -> Self {
        Self {
            color,
            stroke_weight,
            stroke_color,
        }
    }

    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: true,
            color: MurreletColorStyle::color(self.color),
            stroke_weight: self.stroke_weight,
            stroke_color: MurreletColorStyle::color(self.stroke_color),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StyledPathSvgFill {
    pub src: StrId, // for drawing a gpu generated texture
    pub transform: Mat4,
    pub alpha: f32,
    pub width: f32, // % of 100
    pub height: f32,
}

pub fn fixed_pt_f32_to_str(x: f32) -> String {
    FixedPointF32::new(x).to_str()
}

fn find_center_and_size<F: IsPolyline>(points: &F) -> (Vec2, f32, f32) {
    let mut p = points.into_iter_vec2();
    // hmmm
    let s = points
        .into_iter_vec2()
        .fold(Vec2::ZERO, |acc, vec| acc + vec);
    let center = s / p.len() as f32;
    let size = points
        .into_iter_vec2()
        .map(|x| x.distance(center))
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.01);
    let first_loc = p.next().unwrap();
    let first_point = first_loc - center;
    let angle = f32::atan2(first_point.y, first_point.x);

    (center, size, angle)
}

impl StyledPathSvgFill {
    // for nannou
    pub fn to_points_textured<F: IsPolyline>(&self, raw_points: &F) -> Vec<(Vec2, Vec2)> {
        // so using this to center it
        let (center, size, _angle) = find_center_and_size(raw_points);
        let transform = self.transform; // * mat4_from_mat3_transform(Mat3::from_angle(-angle));
        let points = raw_points
            .into_iter_vec2()
            .map(|x| {
                let y = (x - center) / size;
                let z = transform.transform_vec2(y);
                (x, z)
            })
            .collect::<Vec<_>>();

        points
    }

    // for svg
    pub fn hash(&self) -> String {
        let mut hasher = Md5::new();
        hasher.update(self.src.as_str());

        // okay so borrowing from svg, we only care about a few numbers here...
        let [[a, b, _, _], [c, d, _, _], _, [e, f, _, _]] = self.transform.to_cols_array_2d();
        for &v_f32 in &[a, b, c, d, e, f] {
            hasher.update(fixed_pt_f32_to_str(v_f32));
        }
        hasher.update(fixed_pt_f32_to_str(self.alpha));
        hasher.update(fixed_pt_f32_to_str(self.width));
        hasher.update(fixed_pt_f32_to_str(self.height));

        let result = hasher.finalize();

        hex::encode(result)[..6].to_owned()
    }

    pub fn new(src: StrId, transform: Mat4, alpha: f32, width: f32, height: f32) -> Self {
        Self {
            src,
            transform,
            alpha,
            width,
            height,
        }
    }

    pub fn with_alpha(&self, alpha: f32) -> StyledPathSvgFill {
        let mut p = self.clone();
        p.alpha = alpha;
        p
    }
}

#[derive(Clone, Debug, Livecode, Lerpable)]
pub struct MurreletStyleFilledSvg {
    #[livecode(kind = "none")]
    pub pattern_id: String, // reference to canvas
    #[livecode(serde_default = "default")]
    pub transform: Transform2d,
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
    #[livecode(serde_default = "ones")]
    pub alpha: f32,
    #[livecode(serde_default = "ones")]
    pub width: f32,
    #[livecode(serde_default = "ones")]
    pub height: f32,
}
impl MurreletStyleFilledSvg {
    fn to_color_style(&self) -> StyledPathSvgFill {
        StyledPathSvgFill {
            src: StrId::new(&self.pattern_id),
            transform: self.transform.to_mat4(),
            alpha: self.alpha,
            width: self.width,
            height: self.height,
        }
    }

    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: true,
            color: MurreletColorStyle::SvgFill(self.to_color_style()),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, Lerpable, Default)]
pub struct MurreletStyleRGBAFill {
    #[lerpable(func = "lerpify_vec3")]
    pub rgb: Vec3, // red and green, can be negative
    #[livecode(serde_default = "ones")]
    pub a: f32,
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStyleRGBAFill {
    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: true,
            color: MurreletColorStyle::rgbafill(self.rgb, self.a),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, Lerpable, Default)]
pub struct MurreletStyleRGBALine {
    #[lerpable(func = "lerpify_vec3")]
    pub rgb: Vec3, // red and green, can be negative
    #[livecode(serde_default = "ones")]
    pub a: f32,
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStyleRGBALine {
    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: false,
            filled: false,
            color: MurreletColorStyle::rgbafill(self.rgb, self.a),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, Lerpable, Default)]
pub struct MurreletStyleDAFill {
    #[lerpable(func = "lerpify_vec3")]
    pub rgb: Vec3, // red and green, can be negative
    #[livecode(serde_default = "zeros")]
    a: f32,
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStyleDAFill {
    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: true,
            color: MurreletColorStyle::rgbafill(self.rgb, self.a),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, Lerpable, Default)]
pub struct MurreletStyleOutlined {
    pub color: MurreletColor, // fill color
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStyleOutlined {
    pub fn new(color: MurreletColor, stroke_weight: f32) -> Self {
        Self {
            color,
            stroke_weight,
        }
    }

    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: false,
            color: MurreletColorStyle::color(self.color),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }

    fn black() -> MurreletStyleOutlined {
        MurreletStyleOutlined {
            color: MurreletColor::hsva(0.0, 0.0, 0.0, 1.0),
            stroke_weight: 0.0,
        }
    }

    // fn to_style_points(&self) -> MurreletStyle {
    //     MurreletStyle {
    //         points: true,
    //         closed: false,
    //         filled: false,
    //         color: MurreletColorStyle::color(self.color),
    //         stroke_weight: self.stroke_weight,
    //     }
    // }
}

#[derive(Copy, Clone, Debug, Livecode, Lerpable, Default)]
pub struct MurreletStylePoints {
    pub color: MurreletColor, // fill color
    pub shape: PixelShape,
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStylePoints {
    pub fn new(color: MurreletColor, shape: PixelShape, stroke_weight: f32) -> Self {
        Self {
            color,
            shape,
            stroke_weight,
        }
    }

    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            points: Some(self.shape),
            closed: true,
            filled: false,
            color: MurreletColorStyle::color(self.color),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, Lerpable, Default)]
pub struct MurreletStyleRGBAPoints {
    #[lerpable(func = "lerpify_vec3")]
    pub rgb: Vec3, // fill color
    pub a: f32,
    pub shape: PixelShape,
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStyleRGBAPoints {
    // pub fn new(color: MurreletColor, shape: PixelShape, stroke_weight: f32) -> Self {
    //     Self { color, shape, stroke_weight }
    // }

    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            points: Some(self.shape),
            closed: true,
            filled: false,
            color: MurreletColorStyle::rgbafill(self.rgb, self.a),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, MurreletGUI, Lerpable, Default)]
pub struct MurreletStyleLined {
    pub color: MurreletColor, // fill color
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStyleLined {
    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: false,
            filled: false,
            color: MurreletColorStyle::color(self.color),
            stroke_weight: self.stroke_weight,
            ..Default::default()
        }
    }
}

// type DrawingThing<'a, T> = Drawing<'a, T>;

pub mod styleconf {
    use murrelet_livecode_derive::Livecode;

    use super::*;

    // use this one, so you can get the shortcuts
    #[derive(Clone, Debug, Livecode, Lerpable)]
    pub enum StyleConf {
        // Verbose(MurreletStyle),
        Texture(MurreletStyleFilledSvg),
        Fill(MurreletStyleFilled),
        Outline(MurreletStyleOutlined),
        Line(MurreletStyleLined),
        Points(MurreletStylePoints),
        ThickLine,
        RGBAFill(MurreletStyleRGBAFill),
        RGBALine(MurreletStyleRGBALine),
        RGBAOutline(MurreletStyleRGBALine),
        RGBAPoints(MurreletStyleRGBAPoints),
    }
    impl StyleConf {
        pub fn to_style(&self) -> MurreletStyle {
            match self {
                // StyleConf::Verbose(a) => *a,
                StyleConf::Fill(a) => a.to_style(),
                StyleConf::Outline(a) => a.to_style(),
                StyleConf::Line(a) => a.to_style(),
                StyleConf::ThickLine => MurreletStyleOutlined::black().to_style(),
                StyleConf::RGBAFill(a) => a.to_style(),
                StyleConf::RGBALine(a) => a.to_style(),
                StyleConf::RGBAOutline(a) => a.to_style().with_no_fill(),
                StyleConf::Points(a) => a.to_style(),
                StyleConf::RGBAPoints(a) => a.to_style(),
                StyleConf::Texture(a) => a.to_style(),
            }
        }

        pub fn color(&self) -> MurreletColor {
            self.to_style().color.as_color()
        }

        pub fn stroke_weight(&self) -> f32 {
            self.to_style().stroke_weight
        }

        pub fn outline(color: MurreletColor, stroke_weight: f32) -> Self {
            Self::Outline(MurreletStyleOutlined {
                color,
                stroke_weight,
            })
        }

        pub fn font(color: MurreletColor, font_size: f32) -> Self {
            Self::Fill(MurreletStyleFilled {
                color,
                stroke_weight: font_size,
                stroke_color: MurreletColor::transparent(),
            })
        }

        pub fn line(color: MurreletColor, stroke_weight: f32) -> Self {
            Self::Line(MurreletStyleLined {
                color,
                stroke_weight,
            })
        }

        pub fn fill(color: MurreletColor) -> Self {
            Self::Fill(MurreletStyleFilled {
                color,
                stroke_weight: 0.0,
                stroke_color: MurreletColor::transparent(),
            })
        }

        pub fn outlined_fill(
            color: MurreletColor,
            stroke_weight: f32,
            stroke_color: MurreletColor,
        ) -> Self {
            Self::Fill(MurreletStyleFilled {
                color,
                stroke_weight,
                stroke_color,
            })
        }

        pub fn fill_color(&self) -> MurreletColor {
            self.color()
        }
    }

    impl Default for StyleConf {
        fn default() -> Self {
            StyleConf::Fill(MurreletStyleFilled::default())
        }
    }
}

impl CanMakeGUI for StyleConf {
    fn make_gui() -> murrelet_gui::MurreletGUISchema {
        murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Style)
    }
}

// this one attaches a transform to the curve.
// you can _try_ to apply it using to_curve_maker, but this
// will act funny for non-affine
#[derive(Debug, Clone)]
pub struct MurreletCurve {
    cd: CurveDrawer,
    t: Mat4,
}

impl MurreletCurve {
    pub fn new(cd: CurveDrawer) -> Self {
        Self {
            cd,
            t: Mat4::IDENTITY,
        }
    }

    pub fn transform_with_mat4_after(&self, t: Mat4) -> MurreletCurve {
        Self {
            cd: self.cd.clone(),
            t: t * self.t,
        }
    }

    pub fn transform_with_mat4_before(&self, t: Mat4) -> MurreletCurve {
        Self {
            cd: self.cd.clone(),
            t: self.t * t,
        }
    }

    pub fn mat4(&self) -> Mat4 {
        self.t
    }

    pub fn curve(&self) -> &CurveDrawer {
        &self.cd
    }


}

#[derive(Debug, Clone)]
pub enum MurreletPath {
    Polyline(Polyline),
    Curve(MurreletCurve),
    Svg(TransformedSvgShape),
    MaskedCurve(MurreletCurve, MurreletCurve), // first is mask
}
impl MurreletPath {
    pub fn polyline<F: IsPolyline>(path: F) -> Self {
        Self::Polyline(path.as_polyline())
    }

    pub fn curve(cd: CurveDrawer) -> Self {
        Self::Curve(MurreletCurve::new(cd))
    }

    pub fn as_curve(&self) -> MurreletCurve {
        match self {
            MurreletPath::Polyline(c) => MurreletCurve {
                cd: CurveDrawer::new_simple_points(c.clone_to_vec(), false),
                t: Mat4::IDENTITY,
            },
            MurreletPath::Curve(c) => c.clone(),
            MurreletPath::Svg(_) => todo!(),
            MurreletPath::MaskedCurve(_mask, c) => c.clone(),
        }
    }

    pub fn transform_with<T: TransformVec2>(&self, t: &T) -> Self {
        match self {
            MurreletPath::Polyline(x) => MurreletPath::Polyline(x.transform_with(t)),
            MurreletPath::Curve(mc) => todo!(),
            MurreletPath::Svg(_) => todo!(),
            MurreletPath::MaskedCurve(murrelet_curve, murrelet_curve1) => todo!(),
        }
    }

    pub fn transform_with_mat4_after(&self, t: Mat4) -> MurreletPath {
        match self {
            MurreletPath::Polyline(_) => self.transform_with(&t),
            MurreletPath::Curve(c) => MurreletPath::Curve(c.transform_with_mat4_after(t)),
            MurreletPath::Svg(c) => MurreletPath::Svg(c.transform_with_mat4_after(t)),
            MurreletPath::MaskedCurve(mask, curve) => MurreletPath::MaskedCurve(
                mask.transform_with_mat4_after(t),
                curve.transform_with_mat4_after(t),
            ),
        }
    }

    pub fn transform_with_mat4_before(&self, t: Mat4) -> MurreletPath {
        match self {
            MurreletPath::Polyline(_) => self.transform_with(&t),
            MurreletPath::Curve(c) => MurreletPath::Curve(c.transform_with_mat4_before(t)),
            MurreletPath::Svg(c) => MurreletPath::Svg(c.transform_with_mat4_before(t)),
            MurreletPath::MaskedCurve(mask, curve) => MurreletPath::MaskedCurve(
                mask.transform_with_mat4_before(t),
                curve.transform_with_mat4_before(t),
            ),
        }
    }

    pub fn transform(&self) -> Option<Mat4> {
        match self {
            MurreletPath::Polyline(_) => None,
            MurreletPath::Curve(c) => Some(c.t),
            MurreletPath::Svg(c) => Some(c.t),
            MurreletPath::MaskedCurve(_mask, c) => Some(c.t),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MurreletPathAnnotation(Vec<(String, String)>);
impl MurreletPathAnnotation {
    pub fn noop() -> MurreletPathAnnotation {
        Self(vec![])
    }

    pub fn new(annotation: (String, String)) -> Self {
        Self(vec![annotation])
    }

    pub fn vals(&self) -> &Vec<(String, String)> {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct StyledPath {
    pub path: MurreletPath,
    pub style: MurreletStyle,
    pub annotations: MurreletPathAnnotation, // can be useful to attach information to a particular path, for interactions
}
impl StyledPath {
    pub fn new_from_path(path: MurreletPath, style: MurreletStyle) -> Self {
        Self {
            path,
            style,
            annotations: MurreletPathAnnotation::noop(),
        }
    }

    pub fn new_from_path_with_annotation(
        path: MurreletPath,
        style: MurreletStyle,
        annotation: (String, String),
    ) -> Self {
        Self {
            path,
            style,
            annotations: MurreletPathAnnotation::new(annotation),
        }
    }

    pub fn new<F: IsPolyline>(path: F, style: MurreletStyle) -> Self {
        Self {
            path: MurreletPath::Polyline(path.as_polyline()),
            style,
            annotations: MurreletPathAnnotation::noop(),
        }
    }

    pub fn from_path<P: IsPolyline>(path: P) -> StyledPath {
        StyledPath {
            path: MurreletPath::Polyline(path.as_polyline()),
            style: MurreletStyle::default(),
            annotations: MurreletPathAnnotation::noop(),
        }
    }

    pub fn transform_path<T: TransformVec2>(&self, t: &T) -> Self {
        StyledPath {
            path: self.path.transform_with(t),
            ..self.clone()
        }
    }

    pub fn transform_with_mat4_after(&self, t: Mat4) -> Self {
        StyledPath {
            path: self.path.transform_with_mat4_after(t),
            ..self.clone()
        }
    }
}
