#![allow(dead_code)]
use crate::{curve_drawer::CurveDrawer, draw::*, transform2d::*};
use glam::*;
use md5::{Digest, Md5};
use murrelet_common::*;
use murrelet_livecode_derive::{Livecode, UnitCell};

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
pub struct MurreletStyleFilled {
    pub color: MurreletColor, // fill color
    #[livecode(serde_default = "zeros")]
    pub stroke_weight: f32,
}
impl MurreletStyleFilled {
    fn to_style(&self) -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: true,
            color: MurreletColorStyle::color(self.color),
            stroke_weight: self.stroke_weight,
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

impl StyledPathSvgFill {
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

#[derive(Clone, Debug, Livecode, UnitCell, Default)]
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

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
pub struct MurreletStyleRGBAFill {
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

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
pub struct MurreletStyleRGBALine {
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

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
pub struct MurreletStyleDAFill {
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

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
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

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
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
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
pub struct MurreletStyleRGBAPoints {
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
        }
    }
}

#[derive(Copy, Clone, Debug, Livecode, UnitCell, Default)]
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
    use murrelet_livecode_derive::{Livecode, UnitCell};

    use super::*;

    // use this one, so you can get the shortcuts
    #[derive(Clone, Debug, Livecode, UnitCell)]
    pub enum StyleConf {
        // Verbose(MurreletStyle),
        SvgPattern(MurreletStyleFilledSvg),
        Fill(MurreletStyleFilled),
        Outline(MurreletStyleOutlined),
        Points(MurreletStylePoints),
        Line(MurreletStyleLined),
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
                StyleConf::SvgPattern(a) => a.to_style(),
            }
        }

        pub fn color(&self) -> MurreletColor {
            self.to_style().color.as_color()
        }
    }

    impl Default for StyleConf {
        fn default() -> Self {
            StyleConf::Fill(MurreletStyleFilled::default())
        }
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
                cd: CurveDrawer::new_simple_points(c.clone_to_vec()),
                t: Mat4::IDENTITY,
            },
            MurreletPath::Curve(c) => c.clone(),
        }
    }

    pub fn transform_with<T: TransformVec2>(&self, t: &T) -> Self {
        match self {
            MurreletPath::Polyline(x) => MurreletPath::Polyline(x.transform_with(t)),
            MurreletPath::Curve(_) => todo!(), // i'm not sure how i want to handle this yet
        }
    }

    pub fn transform_with_mat4_after(&self, t: Mat4) -> MurreletPath {
        match self {
            MurreletPath::Polyline(_) => self.transform_with(&t),
            MurreletPath::Curve(c) => MurreletPath::Curve(c.transform_with_mat4_after(t)),
        }
    }

    pub fn transform_with_mat4_before(&self, t: Mat4) -> MurreletPath {
        match self {
            MurreletPath::Polyline(_) => self.transform_with(&t),
            MurreletPath::Curve(c) => MurreletPath::Curve(c.transform_with_mat4_before(t)),
        }
    }

    pub fn transform(&self) -> Option<Mat4> {
        match self {
            MurreletPath::Polyline(_) => None,
            MurreletPath::Curve(c) => Some(c.t),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StyledPath {
    pub path: MurreletPath,
    pub style: MurreletStyle,
}
impl StyledPath {
    pub fn new_from_path(path: MurreletPath, style: MurreletStyle) -> Self {
        Self { path, style }
    }

    pub fn new<F: IsPolyline>(path: F, style: MurreletStyle) -> Self {
        Self {
            path: MurreletPath::Polyline(path.as_polyline()),
            style,
        }
    }

    pub fn from_path<P: IsPolyline>(path: P) -> StyledPath {
        StyledPath {
            path: MurreletPath::Polyline(path.as_polyline()),
            style: MurreletStyle::default(),
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
