#![allow(dead_code)]
use geo::{BooleanOps, BoundingRect, Contains};
use glam::{vec2, Vec2};
use itertools::Itertools;
use murrelet_common::SpotOnCurve;
// use leave_common::prelude::*;
use crate::{curve_drawer::{CurveDrawer, ToCurveDrawer}, drawable::DrawnShape};

// pub fn line_to_multipolygon(curves: &[Vec2]) -> geo::MultiPolygon {
//     geo::MultiPolygon::new(vec![line_to_polygon(curves)])
// }

// pub fn line_to_polygon(curves: &[Vec2]) -> geo::Polygon {
//     geo::Polygon::new(vec2_to_line_string(curves), vec![])
// }

// fn vec2_to_line_string(vs: &[Vec2]) -> geo::LineString {
//     let coords = vs.iter().map(|x| x.to_coord()).collect_vec();
//     geo::LineString::new(coords)
// }

// pub fn multipolygon_to_vec2(p: &geo::MultiPolygon) -> Vec<Vec<Vec2>> {
//     p.iter().map(|pp| polygon_to_vec2(pp)).collect_vec()
// }

// pub fn polygon_to_vec2(p: &geo::Polygon) -> Vec<Vec2> {
//     let mut coords = p
//         .exterior()
//         .coords()
//         .into_iter()
//         .map(|coord| coord.to_vec2())
//         .collect_vec();

//     if coords.first() == coords.last() {
//         coords.pop();
//     }

//     coords
// }

// trait ToCoord {
//     fn to_coord(&self) -> ::geo::Coord;
// }

// impl ToCoord for Vec2 {
//     fn to_coord(&self) -> ::geo::Coord {
//         ::geo::coord! {x: self.x as f64, y: self.y as f64}
//     }
// }

// trait ToVec2Griddable {
//     fn to_vec2(&self) -> Vec2;
// }

// impl ToVec2Griddable for ::geo::Coord {
//     fn to_vec2(&self) -> Vec2 {
//         let (x, y) = self.x_y();
//         vec2(x as f32, y as f32)
//     }
// }

// #[derive(Debug, Clone)]
// pub struct MaskCacheImpl {
//     bounding: geo::Rect,
//     polygon: geo::Polygon,
// }
// impl MaskCacheImpl {
//     fn center(&self) -> Vec2 {
//         0.5 * (self.bounding.min().to_vec2() + self.bounding.max().to_vec2())
//     }

//     fn contains(&self, v: &Vec2) -> bool {
//         self.polygon.contains(&v.to_coord())
//     }

//     // left needs to be inside
//     fn last_point_containing(&self, left: &Vec2, right: &Vec2) -> Vec2 {
//         let mut left = *left;
//         let mut right = *right;
//         while left.distance(right) > 0.2 {
//             let midpoint = 0.5 * (left + right);
//             if self.contains(&midpoint) {
//                 left = midpoint;
//             } else {
//                 right = midpoint;
//             }
//         }
//         // finished!
//         0.5 * (left + right)
//     }
// }

// #[derive(Debug, Clone)]
// pub enum MaskCache {
//     Impl(MaskCacheImpl),
//     AlwaysTrue,
// }

// impl MaskCache {
//     fn center(&self) -> Vec2 {
//         match self {
//             MaskCache::Impl(s) => s.center(),
//             MaskCache::AlwaysTrue => Vec2::ZERO,
//         }
//     }

//     pub fn last_point_containing(&self, left: &Vec2, right: &Vec2) -> Vec2 {
//         match self {
//             MaskCache::Impl(s) => s.last_point_containing(left, right),
//             MaskCache::AlwaysTrue => *right,
//         }
//     }

//     pub fn new_vec2(curves: &[Vec2]) -> Self {
//         let polygon = geo::Polygon::new(vec2_to_line_string(curves), vec![]);

//         MaskCache::Impl(MaskCacheImpl {
//             bounding: polygon.bounding_rect().unwrap(),
//             polygon,
//         })
//     }

//     // pub fn new_cd(cd: CurveDrawer) -> Self {
//     //     Self::new(&vec![cd])
//     // }

//     // pub fn new(curves: &[CurveDrawer]) -> Self {
//     //     // first curve is external
//     //     let (first_curve, rest) = curves.split_first().unwrap();
//     //     let first = curve_segment_maker_to_line_string(first_curve);

//     //     let mut remaining = vec![];
//     //     // add all our points to a hashmap
//     //     for curve_maker in rest {
//     //         remaining.push(curve_segment_maker_to_line_string(curve_maker));
//     //     }

//     //     let polygon = ::geo::Polygon::new(first, remaining);

//     //     MaskCache::Impl(MaskCacheImpl {
//     //         bounding: polygon.bounding_rect().unwrap(),
//     //         polygon,
//     //     })
//     // }

//     pub fn contains(&self, v: &Vec2) -> bool {
//         match self {
//             MaskCache::Impl(x) => x.contains(v),
//             MaskCache::AlwaysTrue => true,
//         }
//     }

//     pub fn noop() -> MaskCache {
//         MaskCache::AlwaysTrue
//     }

//     pub fn crop(&self, shape: &[Vec2]) -> Vec<Vec<Vec2>> {
//         match self {
//             MaskCache::Impl(x) => {
//                 let other = line_to_polygon(shape);
//                 let cropped = x.polygon.intersection(&other);
//                 multipolygon_to_vec2(&cropped)
//             }
//             MaskCache::AlwaysTrue => vec![shape.to_vec()],
//         }
//     }

//     // remove this object from all of the shapes
//     pub fn crop_inverse(&self, shape: &[Vec2]) -> Vec<Vec<Vec2>> {
//         match self {
//             MaskCache::Impl(x) => {
//                 let other = line_to_polygon(shape);
//                 let cropped = other.difference(&x.polygon);
//                 multipolygon_to_vec2(&cropped)
//             }
//             MaskCache::AlwaysTrue => vec![shape.to_vec()],
//         }
//     }

//     pub fn to_vec2(&self) -> Vec<Vec2> {
//         match self {
//             MaskCache::Impl(mask_cache_impl) => polygon_to_vec2(&mask_cache_impl.polygon),
//             MaskCache::AlwaysTrue => unreachable!(),
//         }
//     }

//     pub fn crop_line(&self, v: &[Vec2]) -> Vec<Vec<Vec2>> {
//         let mut all_vals = vec![];
//         let mut s = vec![];
//         let mut last_val = None;
//         for c in v.into_iter() {
//             if self.contains(&c) {
//                 last_val = Some(c);
//                 s.push(*c)
//             } else if let Some(x) = last_val {
//                 let last_point_containing = self.last_point_containing(&x, &c);
//                 s.push(last_point_containing);

//                 all_vals.push(s);
//                 last_val = None;
//                 s = vec![];
//             }
//         }
//         if s.len() > 0 {
//             all_vals.push(s);
//         }

//         all_vals
//     }

//     // pub fn crop_many(&self, v: &[DrawnShape]) -> Vec<DrawnShape> {
//     //     let mut cropped = vec![];
//     //     for a in v.into_iter() {
//     //         let mut new_cds = vec![];
//     //         for cd in a.faces() {
//     //             new_cds.extend(self.crop(cd.vertices()));
//     //         }
//     //         cropped.push(DrawnShape::new_vecvec(new_cds, a.style()));
//     //     }
//     //     return cropped;
//     // }

//     // pub fn crop_inverse_many(&self, v: &[DrawnShape]) -> Vec<DrawnShape> {
//     //     let mut cropped = vec![];
//     //     for a in v.into_iter() {
//     //         let mut new_cds = vec![];
//     //         for cd in a.faces() {
//     //             new_cds.extend(self.crop_inverse(cd.vertices()));
//     //         }
//     //         cropped.push(DrawnShape::new_vecvec(new_cds, a.style()));
//     //     }
//     //     return cropped;
//     // }
// }



// use crate::curve::{ToFaces, WithLeaveCurveDrawerMethods};

pub fn line_to_multipolygon(curves: &[Vec2]) -> geo::MultiPolygon {
    geo::MultiPolygon::new(vec![line_to_polygon(curves)])
}

pub fn line_to_polygon(curves: &[Vec2]) -> geo::Polygon {
    geo::Polygon::new(vec2_to_line_string(curves), vec![])
}

pub fn multipolygon_to_vec2(p: &geo::MultiPolygon) -> Vec<Vec<Vec2>> {
    p.iter().map(|pp| polygon_to_vec2(pp)).collect_vec()
}

pub fn polygon_to_vec2(p: &geo::Polygon) -> Vec<Vec2> {
    let mut coords = p
        .exterior()
        .coords()
        .into_iter()
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

fn curve_segment_maker_to_line_string(curve: &CurveDrawer) -> geo::LineString {
    vec2_to_line_string(&curve.to_rough_points(1.0))
}

fn vec2_to_line_string(vs: &[Vec2]) -> geo::LineString {
    let coords = vs.iter().map(|x| x.to_coord()).collect_vec();
    geo::LineString::new(coords)
}

#[derive(Debug, Clone)]
pub enum MaskCache {
    Impl(MaskCacheImpl),
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

    pub fn new_cd(cd: CurveDrawer) -> Self {
        Self::new(&vec![cd])
    }

    pub fn new_interior(outline: CurveDrawer, interior: &[CurveDrawer]) -> Self {
        // shh, just a wrapper
        let s = vec![vec![outline], interior.to_vec()].concat();
        Self::new(&s)

    }

    pub fn new(curves: &[CurveDrawer]) -> Self {
        // first curve is external
        let (first_curve, rest) = curves.split_first().unwrap();
        let first = curve_segment_maker_to_line_string(first_curve);

        let mut remaining = vec![];
        // add all our points to a hashmap
        for curve_maker in rest {
            remaining.push(curve_segment_maker_to_line_string(curve_maker));
        }

        let polygon = ::geo::Polygon::new(first, remaining);

        MaskCache::Impl(MaskCacheImpl {
            bounding: polygon.bounding_rect().unwrap(),
            polygon,
        })
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
        for a in v.into_iter() {
            let mut new_cds = vec![];
            for cd in a.curves() {
                new_cds.extend(self.crop(&cd.to_rough_points(1.0)));
            }
            cropped.push(DrawnShape::new_vecvec(new_cds, a.style()));
        }
        return cropped;
    }

    pub fn crop_inverse_many(&self, v: &[DrawnShape]) -> Vec<DrawnShape> {
        let mut cropped = vec![];
        for a in v.into_iter() {
            let mut new_cds = vec![];
            for cd in a.curves() {
                new_cds.extend(self.crop_inverse(&cd.to_rough_points(1.0)));
            }
            cropped.push(DrawnShape::new_vecvec(new_cds, a.style()));
        }
        return cropped;
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

        // match self {
        //     MaskCache::Impl(mask_cache_impl) => {
        //         mask_cache_impl.
        //
        //     },
        //     MaskCache::AlwaysTrue => todo!(),
        // }
    }
}
