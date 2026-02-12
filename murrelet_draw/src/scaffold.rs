#![allow(dead_code)]
use crate::{
    curve_drawer::{CurveDrawer, ToCurveDrawer},
    drawable::{DrawnShape, ToDrawnShape},
    tesselate::ToLyonPath,
};
use geo::{BooleanOps, BoundingRect, Contains, MultiPolygon};
use glam::{Vec2, vec2};
use itertools::Itertools;

use murrelet_common::SpotOnCurve;
use murrelet_livecode::types::LivecodeResult;

pub fn line_to_multipolygon(curves: &[Vec2]) -> geo::MultiPolygon {
    geo::MultiPolygon::new(vec![line_to_polygon(curves)])
}

pub fn line_to_polygon(curves: &[Vec2]) -> geo::Polygon {
    geo::Polygon::new(vec2_to_line_string(curves), vec![])
}

pub fn multipolygon_to_vec2(p: &geo::MultiPolygon) -> Vec<Vec<Vec2>> {
    p.iter().map(polygon_to_vec2).collect_vec()
}

pub fn polygon_to_vec2(p: &geo::Polygon) -> Vec<Vec2> {
    let mut coords = p
        .exterior()
        .coords()
        .map(|coord| coord.to_vec2())
        .collect_vec();

    if coords.first() == coords.last() {
        coords.pop();
    }

    coords
}

trait ToVec2Griddable {
    fn to_vec2(&self) -> Vec2;
}

impl ToVec2Griddable for ::geo::Coord {
    fn to_vec2(&self) -> Vec2 {
        let (x, y) = self.x_y();
        vec2(x as f32, y as f32)
    }
}

trait ToCoord {
    fn to_coord(&self) -> ::geo::Coord;
}

impl ToCoord for Vec2 {
    fn to_coord(&self) -> ::geo::Coord {
        ::geo::coord! {x: self.x as f64, y: self.y as f64}
    }
}

#[derive(Debug, Clone)]
pub struct MaskCacheImpl {
    bounding: geo::Rect,
    polygon: geo::Polygon,
}
impl MaskCacheImpl {
    fn center(&self) -> Vec2 {
        0.5 * (self.bounding.min().to_vec2() + self.bounding.max().to_vec2())
    }

    fn contains(&self, v: &Vec2) -> bool {
        self.polygon.contains(&v.to_coord())
    }

    // left needs to be inside
    fn last_point_containing(&self, left: &Vec2, right: &Vec2) -> Vec2 {
        let mut left = *left;
        let mut right = *right;
        while left.distance(right) > 0.2 {
            let midpoint = 0.5 * (left + right);
            if self.contains(&midpoint) {
                left = midpoint;
            } else {
                right = midpoint;
            }
        }
        // finished!
        0.5 * (left + right)
    }
}

// fn curve_segment_maker_to_line_string(curve: &CurveDrawer, tolerance: f32) -> LivecodeResult<geo::LineString> {
//     let c = curve.to_lyon()?;
//     vec2_to_line_string(&curve.to_rough_points(1.0))
// }

fn vec2_to_line_string(vs: &[Vec2]) -> geo::LineString {
    let coords = vs.iter().map(|x| x.to_coord()).collect_vec();
    geo::LineString::new(coords)
}

#[derive(Debug, Clone, Default)]
pub enum MaskCache {
    Impl(MaskCacheImpl),
    #[default] // todo, this should be uninitialized or something...
    AlwaysTrue,
}

impl MaskCache {
    pub fn center(&self) -> Vec2 {
        match self {
            MaskCache::Impl(s) => s.center(),
            MaskCache::AlwaysTrue => Vec2::ZERO,
        }
    }

    pub fn last_point_containing(&self, left: &Vec2, right: &Vec2) -> Vec2 {
        match self {
            MaskCache::Impl(s) => s.last_point_containing(left, right),
            MaskCache::AlwaysTrue => *right,
        }
    }

    pub fn new_vec2(curves: &[Vec2]) -> Self {
        let polygon = geo::Polygon::new(vec2_to_line_string(curves), vec![]);

        MaskCache::Impl(MaskCacheImpl {
            bounding: polygon.bounding_rect().unwrap(),
            polygon,
        })
    }

    pub fn new_cd(cd: CurveDrawer, tolerance: f32) -> LivecodeResult<Self> {
        Self::new(&[cd], tolerance)
    }

    pub fn new_interior(
        outline: CurveDrawer,
        interior: &[CurveDrawer],
        tolerance: f32,
    ) -> LivecodeResult<Self> {
        // shh, just a wrapper
        let s = [vec![outline], interior.to_vec()].concat();
        Self::new(&s, tolerance)
    }

    pub fn new(curves: &[CurveDrawer], tolerance: f32) -> LivecodeResult<Self> {
        // first curve is external
        let (first_curve, rest) = curves.split_first().unwrap();
        // let first = curve_segment_maker_to_line_string(first_curve);
        let first = vec2_to_line_string(&first_curve.flatten_with_lyon(tolerance)?);

        let mut remaining = vec![];
        // add all our points to a hashmap
        for curve_maker in rest {
            remaining.push(vec2_to_line_string(
                &curve_maker.flatten_with_lyon(tolerance)?,
            ));
        }

        let polygon = ::geo::Polygon::new(first, remaining);

        Ok(MaskCache::Impl(MaskCacheImpl {
            bounding: polygon.bounding_rect().unwrap(),
            polygon,
        }))
    }

    pub fn contains(&self, v: &Vec2) -> bool {
        match self {
            MaskCache::Impl(x) => x.contains(v),
            MaskCache::AlwaysTrue => true,
        }
    }

    pub fn noop() -> MaskCache {
        MaskCache::AlwaysTrue
    }

    pub fn crop(&self, shape: &[Vec2]) -> Vec<Vec<Vec2>> {
        match self {
            MaskCache::Impl(x) => {
                let other = line_to_polygon(shape);
                let cropped = x.polygon.intersection(&other);
                multipolygon_to_vec2(&cropped)
            }
            MaskCache::AlwaysTrue => vec![shape.to_vec()],
        }
    }

    // remove this object from all of the shapes
    fn crop_inverse(&self, shape: &[Vec2]) -> Vec<Vec<Vec2>> {
        match self {
            MaskCache::Impl(x) => {
                let other = line_to_polygon(shape);
                let cropped = other.difference(&x.polygon);
                multipolygon_to_vec2(&cropped)
            }
            MaskCache::AlwaysTrue => vec![shape.to_vec()],
        }
    }

    pub fn to_vec2(&self) -> Vec<Vec2> {
        match self {
            MaskCache::Impl(mask_cache_impl) => polygon_to_vec2(&mask_cache_impl.polygon),
            MaskCache::AlwaysTrue => unreachable!(),
        }
    }

    pub fn crop_many(&self, v: &[DrawnShape]) -> Vec<DrawnShape> {
        let mut cropped = vec![];
        for a in v.iter() {
            let mut new_cds = vec![];
            for cd in a.curves() {
                new_cds.extend(self.crop(&cd.to_rough_points(1.0)));
            }
            cropped.push(DrawnShape::new_vecvec(new_cds, a.style()));
        }
        cropped
    }

    pub fn crop_inverse_many(&self, v: &[DrawnShape]) -> Vec<DrawnShape> {
        let mut cropped = vec![];
        for a in v.iter() {
            let mut new_cds = vec![];
            for cd in a.curves() {
                new_cds.extend(self.crop_inverse(&cd.to_rough_points(1.0)));
            }
            cropped.push(DrawnShape::new_vecvec(new_cds, a.style()));
        }
        cropped
    }

    pub fn ray_intersect(&self, spot: &SpotOnCurve) -> Vec2 {
        self.last_point_containing(&spot.loc, &spot.line_to_spot(1000.0))
    }
    pub fn rect(&self) -> murrelet_common::Rect {
        match self {
            MaskCache::Impl(mask_cache_impl) => {
                let w = mask_cache_impl.bounding.width();
                let h = mask_cache_impl.bounding.height();
                let center = mask_cache_impl.bounding.center();

                murrelet_common::Rect::from_xy_wh(center.to_vec2(), vec2(w as f32, h as f32))
            }
            MaskCache::AlwaysTrue => todo!(),
        }
    }
}

// should probably move from mask cache to this one...

pub struct Masker {
    mask: MultiPolygon,
}
impl Masker {
    pub fn new() -> Self {
        Self {
            mask: MultiPolygon::new(vec![]),
        }
    }
    pub fn from_vec2(v: &[Vec2]) -> Self {
        let mut s = Self::new();
        s.union_vec2(v);
        s
    }

    pub fn from_cd(v: &CurveDrawer, tolerance: f32) -> Self {
        let mut s = Self::new();
        s.union_cd(v, tolerance);
        s
    }

    pub fn from_many_cd(v: &[CurveDrawer], tolerance: f32) -> Self {
        let mut s = Self::new();
        s.union_many_cds(v, tolerance);
        s
    }

    pub fn union_cd(&mut self, cd: &CurveDrawer, tolerance: f32) {
        self.union_vec2(&cd.flatten_with_lyon(tolerance).unwrap_or_default());
    }

    pub fn union_vec2(&mut self, s: &[Vec2]) {
        if s.len() > 3 {
            let other = line_to_polygon(s);
            self.mask = self.mask.union(&MultiPolygon::new(vec![other]))
        }
    }

    pub fn union_many_vec2(&mut self, s: &[Vec<Vec2>]) {
        for styled in s {
            self.union_vec2(styled);
        }
    }

    pub fn union_many_cds(&mut self, cds: &[CurveDrawer], tolerance: f32) {
        for cd in cds {
            self.union_cd(cd, tolerance);
        }
    }

    // pub fn union_many_styled(&mut self, s: &[DrawnShape], tolerance: f32) {
    //     for styled in s {
    //         for cd in styled.curves() {
    //             self.union_cd(cd, tolerance);
    //         }
    //     }
    // }

    pub fn intersect_vec2(&self, cd: &[Vec2]) -> Vec<Vec<Vec2>> {
        // first mask them...
        let p = MultiPolygon::new(vec![line_to_polygon(cd)]);
        let masked = p.intersection(&self.mask);
        let v = multipolygon_to_vec2(&masked);
        v
    }

    pub fn intersect_many_cds(
        &self,
        cds: &[CurveDrawer],
        tolerance: f32,
    ) -> LivecodeResult<Vec<Vec<Vec2>>> {
        let mut all_the_vecs = vec![];
        for cd in cds {
            // first mask them...

            let p = MultiPolygon::new(vec![line_to_polygon(&cd.flatten_with_lyon(tolerance)?)]);
            let masked = p.intersection(&self.mask);

            let v = multipolygon_to_vec2(&masked);

            all_the_vecs.extend(v);
        }

        Ok(all_the_vecs)
    }

    // pub fn intersect_styled_shapes(
    //     &self,
    //     shapes: &[DrawnShape],
    //     tolerance: f32,
    // ) -> LivecodeResult<Vec<DrawnShape>> {
    //     let mut v = vec![];
    //     for c in shapes {
    //         v.push(self.intersect_many_cds(&c.curves(), c.style(), tolerance)?)
    //     }

    //     Ok(v)
    // }

    pub fn to_vec(&self) -> Vec<Vec<Vec2>> {
        multipolygon_to_vec2(&self.mask)
    }
}

impl ToDrawnShape for Masker {
    fn to_drawn_shape(&self, style: crate::style::styleconf::StyleConf) -> DrawnShape {
        DrawnShape::new_vecvec(self.to_vec(), style)
    }
}
