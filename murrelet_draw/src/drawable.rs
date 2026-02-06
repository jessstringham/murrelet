use glam::Vec2;
use itertools::Itertools;
use murrelet_common::{ToSimpleTransform, Transformable};
use murrelet_livecode::types::LivecodeResult;

use crate::{
    curve_drawer::{CurveDrawer, ToCurveDrawer},
    style::styleconf::StyleConf,
    transform2d::Transform2d,
};

// hm, a new type that attaches a shape to its style, but keeps it agnostic to what is drawing.
#[derive(Clone, Debug)]
pub struct DrawnShape {
    cds: Vec<CurveDrawer>,
    style: StyleConf,
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

    pub fn maybe_transform(&self, transform: &Transform2d) -> LivecodeResult<DrawnShape> {
        let mut new = vec![];
        for c in &self.cds {
            new.push(c.maybe_transform(transform)?);
        }
        Ok(DrawnShape::new_cds(&new, self.style.clone()))
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
        DrawnShape::new_cds(&self.cds.transform_with(t), self.style.clone())
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
}

pub trait ToMixedDrawable {
    fn to_mix_drawable(&self) -> MixedDrawableShape;
}

impl ToMixedDrawable for DrawnShape {
    fn to_mix_drawable(&self) -> MixedDrawableShape {
        MixedDrawableShape::Shape(self.clone())
    }
}

pub trait ToMixedDrawables {
    fn to_mixed_drawables(&self) -> Vec<MixedDrawableShape>;
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
