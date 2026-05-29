use glam::Vec2;
use itertools::Itertools;
use murrelet_common::{MurreletIterHelpers, ToSimpleTransform, Transformable};
use murrelet_livecode::types::LivecodeResult;

use crate::{
    curve_drawer::{CurveDrawer, ToCurveDrawer},
    style::{MurreletPathAnnotation, styleconf::StyleConf},
    transform2d::Transform2d,
};

// hm, a new type that attaches a shape to its style, but keeps it agnostic to what is drawing.
#[derive(Clone, Debug)]
pub struct DrawnShape {
    cds: Vec<CurveDrawer>,
    style: StyleConf,
    annotations: MurreletPathAnnotation,
}

impl DrawnShape {
    pub fn new_vecvec(shape: Vec<Vec<Vec2>>, style: StyleConf) -> DrawnShape {
        let cds = shape
            .into_iter()
            .map(|x| CurveDrawer::new_simple_points(x, true))
            .collect_vec();
        Self::new_cds(&cds, style)
    }

    pub fn new_cds(cds: &[CurveDrawer], style: StyleConf) -> DrawnShape {
        Self {
            cds: cds.to_vec(),
            style: style.clone(),
            annotations: MurreletPathAnnotation::noop(),
        }
    }

    pub fn new_cds_with_annotations(
        cds: &[CurveDrawer],
        style: StyleConf,
        annotations: Vec<(String, String)>,
    ) -> DrawnShape {
        Self {
            cds: cds.to_vec(),
            style,
            annotations: MurreletPathAnnotation::new_many(annotations),
        }
    }

    pub fn style(&self) -> StyleConf {
        self.style.clone()
    }

    pub fn set_style(&mut self, style: StyleConf) {
        self.style = style;
    }

    pub fn curves(&self) -> &[CurveDrawer] {
        &self.cds
    }

    pub fn annotations(&self) -> &MurreletPathAnnotation {
        &self.annotations
    }

    pub fn add_annotation(mut self, key: String, val: String) -> Self {
        self.annotations.add(key, val);
        self
    }

    pub fn maybe_transform(&self, transform: &Transform2d) -> LivecodeResult<DrawnShape> {
        let mut new = vec![];
        for c in &self.cds {
            new.push(c.maybe_transform(transform)?);
        }
        Ok(DrawnShape {
            cds: new,
            style: self.style.clone(),
            annotations: self.annotations.clone(),
        })
    }
}

pub trait ToDrawnShapeSegments {
    fn to_drawn_shape_closed(&self, style: StyleConf) -> DrawnShape;
    fn to_drawn_shape_open(&self, style: StyleConf) -> DrawnShape;

    fn to_drawn_shape_closed_r(&self, style: &StyleConf) -> DrawnShape {
        self.to_drawn_shape_closed(style.clone())
    }

    fn to_drawn_shape_open_r(&self, style: &StyleConf) -> DrawnShape {
        self.to_drawn_shape_open(style.clone())
    }
}

impl<T> ToDrawnShapeSegments for T
where
    T: ToCurveDrawer,
{
    fn to_drawn_shape_closed(&self, style: StyleConf) -> DrawnShape {
        DrawnShape::new_cds(&[self.to_cd_closed()], style)
    }

    fn to_drawn_shape_open(&self, style: StyleConf) -> DrawnShape {
        DrawnShape::new_cds(&[self.to_cd_open()], style)
    }
}

pub trait ToDrawnShape {
    fn to_drawn_shape(&self, style: StyleConf) -> DrawnShape;

    fn to_drawn_shape_r(&self, style: &StyleConf) -> DrawnShape {
        self.to_drawn_shape(style.clone())
    }
}

impl ToDrawnShape for CurveDrawer {
    fn to_drawn_shape(&self, style: StyleConf) -> DrawnShape {
        DrawnShape::new_cds(&[self.clone()], style)
    }
}

impl ToDrawnShape for Vec<CurveDrawer> {
    fn to_drawn_shape(&self, style: StyleConf) -> DrawnShape {
        DrawnShape::new_cds(self, style)
    }
}

impl Transformable for CurveDrawer {
    fn transform_with<T: ToSimpleTransform>(&self, t: &T) -> Self {
        self.maybe_transform(t).unwrap_or_else(|_| self.clone())
    }
}

impl Transformable for DrawnShape {
    fn transform_with<T: ToSimpleTransform>(&self, t: &T) -> Self {
        DrawnShape {
            cds: self.cds.transform_with(t),
            style: self.style.clone(),
            annotations: self.annotations.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PositionedText {
    text: String,
    loc: Vec2,
}
impl PositionedText {
    pub fn new(text: &str, loc: Vec2) -> Self {
        Self {
            text: text.to_string(),
            loc,
        }
    }

    pub fn with_style(&self, style: &StyleConf) -> MixedDrawableShape {
        MixedDrawableShape::Text(DrawnTextShape {
            text: vec![self.clone()],
            style: style.clone(),
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn loc(&self) -> Vec2 {
        self.loc
    }
}

impl Transformable for PositionedText {
    fn transform_with<T: ToSimpleTransform>(&self, t: &T) -> Self {
        Self {
            loc: self.loc.transform_with(t),
            ..self.clone()
        }
    }
}

impl Transformable for MixedDrawableShape {
    fn transform_with<T: ToSimpleTransform>(&self, t: &T) -> Self {
        match self {
            MixedDrawableShape::Shape(d) => MixedDrawableShape::Shape(d.transform_with(t)),
            MixedDrawableShape::Text(d) => MixedDrawableShape::Text(d.transform_with(t)),
        }
    }
}

impl ToMixedDrawableWithStyle for Vec<PositionedText> {
    fn with_style(&self, style: &StyleConf) -> MixedDrawableShape {
        MixedDrawableShape::Text(DrawnTextShape {
            text: self.clone(),
            style: style.clone(),
        })
    }
}

impl ToMixedDrawableWithStyle for Vec<CurveDrawer> {
    fn with_style(&self, style: &StyleConf) -> MixedDrawableShape {
        MixedDrawableShape::Shape(self.to_drawn_shape_r(style))
    }
}

pub trait ToMixedDrawableWithStyle {
    fn with_style(&self, style: &StyleConf) -> MixedDrawableShape;
}

#[derive(Clone, Debug)]
pub struct DrawnTextShape {
    text: Vec<PositionedText>,
    style: StyleConf,
}
impl DrawnTextShape {
    pub fn positions(&self) -> &[PositionedText] {
        &self.text
    }
}
impl Transformable for DrawnTextShape {
    fn transform_with<T: ToSimpleTransform>(&self, t: &T) -> Self {
        Self {
            text: self.text.map_iter_collect(|x| x.transform_with(t)),
            ..self.clone()
        }
    }
}

// ergh, need another type to hold type...
#[derive(Clone, Debug)]
pub enum MixedDrawableShape {
    Shape(DrawnShape),
    Text(DrawnTextShape),
}
impl MixedDrawableShape {
    pub fn style(&self) -> StyleConf {
        match self {
            MixedDrawableShape::Shape(drawn_shape) => drawn_shape.style(),
            MixedDrawableShape::Text(drawn_text_shape) => drawn_text_shape.style.clone(),
        }
    }

    pub fn new_from_path_with_multiple_annotations(
        cds: Vec<CurveDrawer>,
        style: StyleConf,
        annotations: Vec<(String, String)>,
    ) -> Self {
        MixedDrawableShape::Shape(DrawnShape::new_cds_with_annotations(
            &cds,
            style,
            annotations,
        ))
    }
}

pub trait ToMixedDrawable {
    fn to_mix_drawable(&self) -> MixedDrawableShape;
}

impl ToMixedDrawable for DrawnShape {
    fn to_mix_drawable(&self) -> MixedDrawableShape {
        MixedDrawableShape::Shape(self.clone())
    }
}

/// Which medium a draw is headed for. Threaded through the draw seam so a
/// sketch CAN return different geometry for the on-screen window vs the
/// svg/plotter output (e.g. filled circles on screen, continuous polylines
/// for a pen). Sketches that don't care never see it (see the default
/// `ToMixedDrawables::to_mixed_drawables_for`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawTarget {
    /// The on-screen nannou window (and, today, the interactive web view).
    Screen,
    /// SVG output — headless svg render or plotter.
    Svg,
}

pub trait ToMixedDrawables {
    fn to_mixed_drawables(&self) -> Vec<MixedDrawableShape>;

    /// Target-aware entry point. Defaults to `to_mixed_drawables()`, so a
    /// sketch that doesn't override this is behaviorally identical for every
    /// target. Override this (and keep `to_mixed_drawables` for the
    /// target-agnostic / Screen case) to diverge screen-vs-svg — e.g.:
    ///
    /// ```ignore
    /// fn to_mixed_drawables_for(&self, target: DrawTarget) -> Vec<MixedDrawableShape> {
    ///     match target {
    ///         DrawTarget::Svg => self.vein_polylines(),  // pen wants strokes
    ///         DrawTarget::Screen => self.vein_circles(),  // filled dots on screen
    ///     }
    /// }
    /// ```
    fn to_mixed_drawables_for(&self, _target: DrawTarget) -> Vec<MixedDrawableShape> {
        self.to_mixed_drawables()
    }
}

impl ToMixedDrawables for MixedDrawableShape {
    fn to_mixed_drawables(&self) -> Vec<MixedDrawableShape> {
        vec![self.clone()]
    }
}

impl ToMixedDrawables for Vec<MixedDrawableShape> {
    fn to_mixed_drawables(&self) -> Vec<MixedDrawableShape> {
        self.clone()
    }
}

impl ToMixedDrawables for DrawnShape {
    fn to_mixed_drawables(&self) -> Vec<MixedDrawableShape> {
        vec![MixedDrawableShape::Shape(self.clone())]
    }
}

impl ToMixedDrawables for Vec<DrawnShape> {
    fn to_mixed_drawables(&self) -> Vec<MixedDrawableShape> {
        let mut v = vec![];
        for x in self.iter() {
            v.push(MixedDrawableShape::Shape(x.clone()));
        }
        v
    }
}
