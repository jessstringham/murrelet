// traits, that you can connect to things like Nannou draw

use glam::{vec2, Mat4, Vec2, Vec3};
use itertools::Itertools;
use murrelet_common::{IsPolyline, MurreletColor, Polyline, TransformVec2};
use murrelet_livecode::unitcells::UnitCellContext;
use murrelet_livecode_derive::Livecode;
use palette::{named::AQUAMARINE, LinSrgba, Srgb};

use crate::{
    curve_drawer::CurveDrawer,
    newtypes::RGBandANewtype,
    style::{styleconf::*, StyledPathSvgFill},
};

#[derive(Debug, Clone, Copy, Livecode, Default)]
#[livecode(enum_tag = "external")]
pub enum PixelShape {
    #[default]
    X,
    O,
    P,
    L,
}

#[derive(Copy, Clone, Debug)]
pub enum MurreletColorStyle {
    Color(MurreletColor),
    RgbaFill(RGBandANewtype),
    SvgFill(StyledPathSvgFill),
}
impl Default for MurreletColorStyle {
    fn default() -> Self {
        MurreletColorStyle::black()
    }
}
impl MurreletColorStyle {
    pub fn color(c: MurreletColor) -> MurreletColorStyle {
        MurreletColorStyle::Color(c)
    }

    pub fn from_srgb_u8(c: Srgb<u8>) -> MurreletColorStyle {
        Self::color(MurreletColor::from_srgb_u8(c))
    }

    pub fn rgbafill(v: Vec3, a: f32) -> MurreletColorStyle {
        MurreletColorStyle::RgbaFill(RGBandANewtype::new(v, a))
    }

    pub fn to_rgba(&self) -> [f32; 4] {
        self.as_color().into_rgba_components()
    }

    pub fn as_color(&self) -> MurreletColor {
        match self {
            MurreletColorStyle::Color(c) => c.clone(),
            MurreletColorStyle::RgbaFill(c) => c.color(),
            MurreletColorStyle::SvgFill(_) => MurreletColor::white(), // default if we're not drawing the texture
        }
    }

    pub fn to_linsrgba(&self) -> LinSrgba {
        self.as_color().to_linsrgba()
    }

    fn white() -> MurreletColorStyle {
        MurreletColorStyle::Color(MurreletColor::white())
    }

    fn black() -> MurreletColorStyle {
        MurreletColorStyle::Color(MurreletColor::black())
    }

    fn with_alpha(&self, alpha: f32) -> MurreletColorStyle {
        match self {
            MurreletColorStyle::Color(c) => MurreletColorStyle::Color(c.with_alpha(alpha)),
            MurreletColorStyle::RgbaFill(c) => MurreletColorStyle::RgbaFill(c.with_alpha(alpha)),
            MurreletColorStyle::SvgFill(c) => MurreletColorStyle::SvgFill(c.with_alpha(alpha)),
        }
    }
}

pub enum MurreletDrawPlan {
    Shader(StyledPathSvgFill),
    DebugPoints(PixelShape),
    FilledClosed,
    Outline,
    Line,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MurreletStyle {
    pub points: Option<PixelShape>,
    pub closed: bool,
    pub filled: bool,
    pub color: MurreletColorStyle, // if filled, fill, otherwise stroke. only for draw.
    pub stroke_weight: f32,
}
impl MurreletStyle {
    pub fn new(
        closed: bool,
        filled: bool,
        color: MurreletColor,
        stroke_weight: f32,
    ) -> MurreletStyle {
        Self {
            closed,
            filled,
            color: MurreletColorStyle::color(color),
            stroke_weight,
            ..Default::default()
        }
    }

    pub fn drawing_plan(&self) -> MurreletDrawPlan {
        if let Some(pt) = &self.points {
            MurreletDrawPlan::DebugPoints(*pt)
        } else if let MurreletColorStyle::SvgFill(s) = self.color {
            MurreletDrawPlan::Shader(s)
        } else if self.closed {
            if self.filled {
                MurreletDrawPlan::FilledClosed
            } else {
                MurreletDrawPlan::Outline
            }
        } else {
            MurreletDrawPlan::Line
        }
    }

    pub fn new_outline() -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: false,
            color: MurreletColorStyle::white(),
            stroke_weight: 1.0,
            ..Default::default()
        }
    }

    pub fn default_guide() -> MurreletStyle {
        MurreletStyle {
            closed: true,
            filled: false,
            color: MurreletColorStyle::from_srgb_u8(AQUAMARINE),
            stroke_weight: 2.0,
            ..Default::default()
        }
    }

    pub fn new_white(closed: bool, filled: bool) -> MurreletStyle {
        MurreletStyle {
            closed,
            filled,
            color: MurreletColorStyle::white(),
            stroke_weight: 0.5,
            ..Default::default()
        }
    }

    pub fn with_color(&self, c: MurreletColor) -> MurreletStyle {
        MurreletStyle {
            color: MurreletColorStyle::color(c),
            ..*self
        }
    }

    pub fn new_fill_color(c: MurreletColor) -> MurreletStyle {
        MurreletStyle {
            points: None,
            color: MurreletColorStyle::color(c),
            closed: true,
            filled: true,
            stroke_weight: 0.0,
        }
    }

    pub fn with_font_weight(&self, w: u32) -> MurreletStyle {
        self.with_stroke(w as f32)
    }

    pub fn with_stroke(&self, w: f32) -> MurreletStyle {
        MurreletStyle {
            stroke_weight: w,
            ..*self
        }
    }

    pub fn with_no_fill(&self) -> MurreletStyle {
        MurreletStyle {
            filled: false,
            ..*self
        }
    }

    pub fn with_open(&self) -> MurreletStyle {
        MurreletStyle {
            closed: false,
            ..*self
        }
    }

    pub fn with_closed(&self) -> MurreletStyle {
        MurreletStyle {
            closed: true,
            ..*self
        }
    }

    pub fn with_svg_fill(&self, fill: StyledPathSvgFill) -> MurreletStyle {
        MurreletStyle {
            color: MurreletColorStyle::SvgFill(fill),
            ..*self
        }
    }
}

pub trait Sdraw: Sized {
    fn with_style(&self, style: StyleConf) -> Self {
        self.with_svg_style(style.to_style())
    }

    fn with_svg_style(&self, svg_style: MurreletStyle) -> Self;
    fn svg_style(&self) -> MurreletStyle;

    fn with_color(&self, color: MurreletColor) -> Self {
        self.with_svg_style(self.svg_style().with_color(color))
    }

    fn with_stroke_weight(&self, w: f32) -> Self {
        self.with_svg_style(self.svg_style().with_stroke(w))
    }

    fn with_close(&self, closed: bool) -> Self {
        let svg_style = MurreletStyle {
            closed,
            ..self.svg_style()
        };
        self.with_svg_style(svg_style)
    }

    fn as_closed(&self) -> Self
    where
        Self: Sized,
    {
        self.with_close(true)
    }

    fn as_opened(&self) -> Self
    where
        Self: Sized,
    {
        self.with_close(false)
    }

    fn with_fill(&self) -> Self {
        let svg_style = MurreletStyle {
            filled: true,
            closed: true,
            ..self.svg_style()
        };
        self.with_svg_style(svg_style)
    }

    fn as_outline(&self) -> Self {
        let svg_style = MurreletStyle {
            filled: false,
            closed: true,
            ..self.svg_style()
        };
        self.with_svg_style(svg_style)
    }

    fn transform(&self) -> Mat4;
    fn set_transform(&self, m: Mat4) -> Self;

    fn add_transform_after(&self, t: Mat4) -> Self {
        let m = t * self.transform();
        self.set_transform(m)
    }

    fn add_transform_before(&self, t: Mat4) -> Self {
        let m = self.transform() * t;
        self.set_transform(m)
    }

    fn transform_points<F: IsPolyline>(&self, face: &F) -> Polyline;

    fn maybe_transform_vec2(&self, v: Vec2) -> Option<Vec2> {
        self.transform_points(&vec![v])
            .into_iter_vec2()
            .collect_vec()
            .first()
            .cloned()
    }

    fn line_space_multi(&self) -> f32;
}

#[derive(Clone, Debug)]
pub struct CoreSDrawCtxUnitCell {
    unit_cell: UnitCellContext,
    unit_cell_skew: bool, // when doing unitcell, whether to keep porportions or not
    sdraw: CoreSDrawCtx,
}
impl Sdraw for CoreSDrawCtxUnitCell {
    fn transform_points<F: IsPolyline>(&self, face: &F) -> Polyline {
        // let mut points = face.clone();

        let mut points = self.sdraw.transform.transform_many_vec2(face);

        // main difference! apply the unit cell transform
        points = if self.unit_cell_skew {
            self.unit_cell.transform_with_skew(&points)
        } else {
            self.unit_cell.transform_no_skew(&points)
        };

        points
    }

    fn with_svg_style(&self, svg_style: MurreletStyle) -> Self {
        Self {
            sdraw: self.sdraw.with_svg_style(svg_style),
            ..self.clone()
        }
    }

    fn svg_style(&self) -> MurreletStyle {
        self.sdraw.svg_style()
    }

    fn line_space_multi(&self) -> f32 {
        self.sdraw.line_space_multi()
    }

    // this only changes the global transform, the unitcell one will still happen
    fn transform(&self) -> Mat4 {
        self.sdraw.transform
    }

    fn set_transform(&self, m: Mat4) -> Self {
        let mut ctx = self.clone();
        ctx.sdraw.transform = m;
        ctx
    }
}

impl CoreSDrawCtxUnitCell {
    pub fn rect_bound(&self) -> Vec<Vec2> {
        self.unit_cell.rect_bound()
    }

    pub fn clear_context(&self) -> CoreSDrawCtx {
        self.sdraw.clone()
    }

    pub fn with_unit_cell_skew(&self, skew: bool) -> Self {
        let mut ctx = self.clone();
        ctx.unit_cell_skew = skew;
        ctx
    }

    pub fn unit_cell(&self) -> &UnitCellContext {
        &self.unit_cell
    }

    pub fn as_outline(&self) -> Self {
        let mut svg = self.clone();
        svg.sdraw.svg_style.filled = false;
        svg
    }

    pub fn unit_cell_skew(&self) -> bool {
        self.unit_cell_skew
    }

    pub fn unit_cell_transform(&self) -> Mat4 {
        if self.unit_cell_skew {
            self.unit_cell.transform_with_skew_mat4()
        } else {
            self.unit_cell.transform_no_skew_mat4()
        }
    }

    pub fn with_alpha(&self, alpha: f32) -> CoreSDrawCtxUnitCell {
        let mut core = self.clone();
        core.sdraw.svg_style.color = core.sdraw.svg_style.color.with_alpha(alpha);
        core
    }
}

// just packages things up so it's easier to use
#[derive(Clone, Debug)]
pub struct CoreSDrawCtx {
    svg_style: MurreletStyle,
    pub frame: f32,
    transform: Mat4, // happens before the others
}
impl Sdraw for CoreSDrawCtx {
    fn transform_points<F: IsPolyline>(&self, face: &F) -> Polyline {
        self.transform.transform_many_vec2(face)
    }

    fn with_svg_style(&self, svg_style: MurreletStyle) -> Self {
        let mut ctx = self.clone();
        ctx.svg_style = svg_style;
        ctx
    }

    fn transform(&self) -> Mat4 {
        self.transform
    }
    fn set_transform(&self, m: Mat4) -> Self {
        let mut sdraw = self.clone();
        sdraw.transform = m;
        sdraw
    }

    fn add_transform_after(&self, t: Mat4) -> Self {
        let mut ctx = self.clone();
        ctx.transform = t * ctx.transform;
        ctx
    }

    fn add_transform_before(&self, t: Mat4) -> Self {
        let mut ctx = self.clone();
        ctx.transform *= t;
        ctx
    }

    fn line_space_multi(&self) -> f32 {
        let first = self.maybe_transform_vec2(vec2(0.0, 0.0));
        let last = self.maybe_transform_vec2(vec2(1.0, 0.0));

        // let points = self.transform_points(&[vec2(0.0, 0.0), vec2(1.0, 0.0)]);
        // 1.0 / points[1].distance(points[0])

        if let (Some(f), Some(l)) = (first, last) {
            1.0 / f.distance(l)
        } else {
            1.0 // eh
        }
    }

    fn svg_style(&self) -> MurreletStyle {
        self.svg_style
    }
}

impl CoreSDrawCtx {
    pub fn new(svg_style: MurreletStyle, frame: f32, transform: Mat4) -> Self {
        Self {
            svg_style,
            frame,
            transform,
        }
    }

    // turning into unit-cell level
    pub fn with_detail(&self, detail: &UnitCellContext) -> CoreSDrawCtxUnitCell {
        CoreSDrawCtxUnitCell {
            unit_cell: detail.clone(),
            unit_cell_skew: false,
            sdraw: self.clone(),
        }
    }
}
