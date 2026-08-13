#![allow(dead_code)]
// started as masking things, but basically geo things!
use crate::{
    convert::glam_to_geo,
    curve_drawer::{CurveDrawer, ToCurveDrawer},
    drawable::{DrawnShape, ToDrawnShape},
    tesselate::ToLyonPath,
};
use ::geo::{
    Buffer,
    buffer::{BufferStyle, LineJoin},
};
use geo::{Area, BooleanOps, BoundingRect, Contains, Intersects, Line, MultiPolygon};
use glam::{Vec2, vec2};
use itertools::Itertools;

use murrelet_common::{MurreletString, PointToPoint, SpotOnCurve, poly_area_unsigned};
use murrelet_livecode::types::{LivecodeResult, ToLivecodeResult};
use std::collections::{HashMap, HashSet};

use crate::convert::glam_to_geo_pt;
use geo::Point;

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

// useful after polygon...
pub fn rings_to_cds(rings: &[Vec<Vec2>], min_area: f32) -> Vec<CurveDrawer> {
    rings
        .iter()
        .filter(|r| r.len() >= 3 && poly_area_unsigned(r) >= min_area)
        .map(|r| r.to_cd_closed())
        .collect()
}

pub fn mp_to_cds(rings: &MultiPolygon, min_area: f32) -> Vec<CurveDrawer> {
    rings_to_cds(&multipolygon_to_vec2(&rings), min_area)
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
        self.polygon.contains(&glam_to_geo(*v))
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
    let coords = vs.iter().map(|x| glam_to_geo(*x)).collect_vec();
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

#[derive(Debug, Clone)]
pub struct Masker {
    mask: MultiPolygon,
}
impl Default for Masker {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn union(&self, other: &Self) -> Self {
        Self {
            mask: self.mask.union(&other.mask),
        }
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

    pub fn contains(&self, v: &Vec2) -> bool {
        self.mask.contains(&glam_to_geo(*v))
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

        multipolygon_to_vec2(&masked)
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

    pub fn has_intersection_with_segment(&self, p2p: PointToPoint) -> bool {
        let line = Line::new(glam_to_geo(p2p.start()), glam_to_geo(p2p.end()));

        self.mask.intersects(&line)
    }

    pub fn difference_area(&self, other: &Masker) -> f32 {
        self.mask.difference(&other.mask).unsigned_area() as f32
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

    pub fn clear(&mut self) {
        self.mask.0.clear();
    }
}

impl ToDrawnShape for Masker {
    fn to_drawn_shape(&self, style: crate::style::styleconf::StyleConf) -> DrawnShape {
        DrawnShape::new_vecvec(self.to_vec(), style)
    }
}

pub struct OffsetConf {
    pub flatten_tolerance: f32,
    pub miter: f32,
}

pub fn offset_cd(cd: &CurveDrawer, distance: f32, conf: &OffsetConf) -> LivecodeResult<Vec<Vec2>> {
    let points = cd.flatten_with_lyon(conf.flatten_tolerance)?;

    offset_outline(&points, distance, conf.miter, cd.closed)
}

pub fn offset_outline(
    points: &[Vec2],
    distance: f32,
    miter: f32,
    closed: bool,
) -> LivecodeResult<Vec<Vec2>> {
    if points.len() < 2 {
        return Err("not enough points").to_lc_err();
    }

    if closed {
        offset_outline_closed(points, distance, miter)
    } else {
        offset_outline_open(points, distance, miter)
    }
}

fn offset_outline_closed(points: &[Vec2], distance: f32, miter: f32) -> LivecodeResult<Vec<Vec2>> {
    if points.len() < 2 {
        return Err("not enough points").to_lc_err();
    }
    let mut points = points.to_vec();

    if points.last().unwrap() != points.first().unwrap() {
        points.push(*points.first().unwrap());
    }

    let mut poly = line_to_polygon(&points);

    // make sure it's facing the right way
    if poly.signed_area() < 0.0 {
        let mut rev = points.to_vec();
        rev.reverse();
        poly = line_to_polygon(&rev);
    }

    let buffer_style = BufferStyle::new(distance as f64).line_join(LineJoin::Miter(miter as f64));
    let grown = poly.buffer_with_style(buffer_style);

    if grown.0.is_empty() {
        return Err("expand shape failed").to_lc_err(); // fallback to original
    }

    // some help
    let largest = grown
        .iter()
        .max_by(|a, b| {
            a.unsigned_area()
                .partial_cmp(&b.unsigned_area())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("buffer produced no polygons");

    Ok(polygon_to_vec2(largest))
}

fn offset_outline_open(points: &[Vec2], distance: f32, miter: f32) -> LivecodeResult<Vec<Vec2>> {
    if points.len() < 2 {
        return Err("not enough points").to_lc_err();
    }

    // Offset 0 is the curve itself -- geo cannot buffer a line by 0.
    if distance.abs() < 1.0e-6 {
        return Ok(points.to_vec());
    }

    let line_string = vec2_to_line_string(points);

    // geo's line buffer needs a positive width; the sign of `distance` only
    // selects which side's rail to keep (see `want_left` below).
    let buffer_style = BufferStyle::new(distance.abs() as f64)
        .line_join(LineJoin::Miter(miter as f64))
        .line_cap(geo::buffer::LineCap::Square);
    let grown = line_string.buffer_with_style(buffer_style);

    if grown.0.is_empty() {
        return Err("expand shape failed").to_lc_err(); // fallback to original
    }

    let largest = grown
        .iter()
        .max_by(|a, b| {
            a.unsigned_area()
                .partial_cmp(&b.unsigned_area())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("buffer produced no polygons");

    // some claude help, so now we need to take this apart. it sounds like we can't
    // get it out of geo or i_offset
    let ring = polygon_to_vec2(largest);
    // signed side of `v` vs the path: >0 = left of path direction, <0 = right
    let path_side = |v: Vec2| -> f32 {
        let mut best_d2 = f32::INFINITY;
        let mut best = 0.0;
        for w in points.windows(2) {
            let p2p = PointToPoint::new(w[0], w[1]);

            if p2p.length() < 1.0e-8 {
                continue;
            }

            let closest = p2p.closest_pt_to_segment(v).end();

            let d2 = v.distance_squared(closest);
            if d2 < best_d2 {
                best_d2 = d2;
                best = p2p.side_of(v);
            }
        }
        best
    };

    // keep the longest contiguous run of boundary vertices on the chosen side
    let want_left = distance >= 0.0;
    let n = ring.len();
    let on_side: Vec<bool> = ring
        .iter()
        .map(|&v| (path_side(v) > 0.0) == want_left)
        .collect();
    let mut best_start = 0;
    let mut best_len = 0;
    let mut i = 0;
    while i < n {
        if on_side[i] {
            let mut run_len = 0;
            while run_len < n && on_side[(i + run_len) % n] {
                run_len += 1;
            }
            if run_len > best_len {
                best_start = i;
                best_len = run_len;
            }
            i += run_len;
        } else {
            i += 1;
        }
    }

    if best_len == 0 {
        return Err("offset: no rail on requested side").to_lc_err();
    }

    let rail: Vec<Vec2> = (0..best_len).map(|k| ring[(best_start + k) % n]).collect();
    Ok(rail)
}

// this will convert things to a list of list of vec2s.
// there are two kinds: a boolean operation that combines shapes
// and then select, this fines the smallest region containing the provided point.
// originally written by claude and then tidied up by me!

// remove things that are too small
const MIN_UNIVERSE_AREA: f32 = 1e-2;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegionName(String);
impl RegionName {
    pub fn to_string(&self) -> &str {
        self.0.as_str()
    }

    pub fn new(n: &str) -> Self {
        Self(n.to_string())
    }
}

#[derive(Clone)]
pub struct RegionList(Vec<RegionName>);
impl RegionList {
    pub fn new(region_names: Vec<RegionName>) -> Self {
        Self(region_names)
    }

    pub fn regions(&self) -> &Vec<RegionName> {
        &self.0
    }

    pub fn new_from_mstrings(s: &[MurreletString]) -> Self {
        RegionList::new(s.iter().map(|n| RegionName::new(&n.to_string())).collect())
    }
}

pub enum RegionKindBool {
    Union(RegionList),
    Intersection(RegionList),
    Difference(RegionList),
    Xor(RegionList),
}
impl RegionKindBool {
    fn members(&self) -> RegionList {
        match &self {
            RegionKindBool::Union(region_list) => region_list.clone(),
            RegionKindBool::Intersection(region_list) => region_list.clone(),
            RegionKindBool::Difference(region_list) => region_list.clone(),
            RegionKindBool::Xor(region_list) => region_list.clone(),
        }
    }

    fn eval(
        &self,
        universe: &RegionUniverse,
        shape_scope: &ShapeScope,
    ) -> LivecodeResult<MultiPolygon> {
        let operands = universe.lookup_many(&self.members(), shape_scope)?;

        let mut operand_iter = operands.into_iter();

        if let Some(first) = operand_iter.next() {
            let mut acc = first.clone();

            for mp in operand_iter {
                acc = match &self {
                    RegionKindBool::Union(_) => acc.union(mp),
                    RegionKindBool::Intersection(_) => acc.intersection(mp),
                    RegionKindBool::Difference(_) => acc.difference(mp),
                    RegionKindBool::Xor(_) => acc.xor(mp),
                };
            }
            Ok(acc)
        } else {
            Ok(MultiPolygon::new(vec![]))
        }
    }
}

pub enum RegionKindSelectPoint {
    ByCoord(Vec2),
    ByPoint(String),
}
impl RegionKindSelectPoint {
    fn to_pt(&self, universe: &RegionUniverse) -> LivecodeResult<Vec2> {
        match self {
            RegionKindSelectPoint::ByCoord(vec2) => Ok(*vec2),
            RegionKindSelectPoint::ByPoint(name) => universe
                .lookup_point(name)
                .map(|x| *x)
                .ok_or_else(|| "missing name")
                .to_lc_err(),
        }
    }
}

pub enum RegionKindSelect {
    ByRegion {
        inside: RegionList,
        outside: RegionList,
    },
    ByLoc(RegionKindSelectPoint),
}
impl RegionKindSelect {
    pub fn empty() -> Self {
        RegionKindSelect::ByRegion {
            inside: RegionList::new(vec![]),
            outside: RegionList::new(vec![]),
        }
    }

    pub fn from_loc_coord(loc: Vec2) -> Self {
        RegionKindSelect::ByLoc(RegionKindSelectPoint::ByCoord(loc))
    }

    pub fn from_region(inside: RegionList, outside: RegionList) -> Self {
        RegionKindSelect::ByRegion { inside, outside }
    }

    pub fn eval(
        &self,
        universe: &RegionUniverse,
        shape_scope: &ShapeScope,
    ) -> LivecodeResult<MultiPolygon> {
        let (marker, inside_region_list, outside_region_list) = match self {
            RegionKindSelect::ByRegion { inside, outside } => {
                (None, inside.clone(), outside.clone())
            }
            RegionKindSelect::ByLoc(pt) => {
                let v = pt.to_pt(universe)?;
                let pt = glam_to_geo_pt(v);
                let (ins, outs) = universe.get_in_out(pt);
                (Some(v), ins, outs)
            }
        };

        let inside = universe
            .lookup_many(&inside_region_list, shape_scope)?
            .clone();
        let outside = universe
            .lookup_many(&outside_region_list, shape_scope)?
            .clone();

        let mut inside_iter = inside.into_iter();

        let mut acc = if let Some(n) = inside_iter.next() {
            n.clone()
        } else {
            return Ok(MultiPolygon::new(vec![]));
        };

        for mp in inside_iter {
            acc = acc.intersection(mp);
        }

        for mp in outside.into_iter() {
            acc = acc.difference(mp);
        }

        // if there are multiple polygons, find the nearest one...
        if let Some(m) = marker {
            if acc.0.len() > 1 {
                if let Some(poly) = acc.0.iter().find(|p| p.contains(&glam_to_geo_pt(m))) {
                    return Ok(MultiPolygon::new(vec![poly.clone()]));
                }
            }
        }
        // hrm, what to do if there are multiple but no marker??
        Ok(acc)
    }
}

pub enum RegionKind {
    Bool(RegionKindBool),
    Select(RegionKindSelect),
}

pub enum ShapeScope {
    All,
    LimitTo(HashSet<String>),   // filter to these
    AllExcept(HashSet<String>), // do all except for
}
impl ShapeScope {
    pub fn all_except(s: HashSet<String>) -> Self {
        Self::AllExcept(s)
    }

    pub fn limit_to(s: HashSet<String>) -> Self {
        Self::LimitTo(s)
    }

    fn allows_for(&self, m: &RegionName) -> bool {
        match self {
            ShapeScope::All => true,
            ShapeScope::LimitTo(items) => items.contains(&m.to_string().to_owned()),
            ShapeScope::AllExcept(items) => !items.contains(&m.to_string().to_owned()),
        }
    }
}

/// One region to resolve. If `name` is set, the region's output is registered
/// under that name so a later region can reference it as a member.
pub struct RegionSpec {
    pub kind: RegionKind,
    pub name: String,
    pub shape_scope: ShapeScope,
}
impl RegionSpec {
    fn eval(&self, universe: &RegionUniverse) -> LivecodeResult<MultiPolygon> {
        match &self.kind {
            RegionKind::Bool(region_kind_bool) => {
                region_kind_bool.eval(universe, &self.shape_scope)
            }
            RegionKind::Select(region_kind_select) => {
                region_kind_select.eval(universe, &self.shape_scope)
            }
        }
    }
}

pub struct RegionUniverseConf {
    pub tolerance: f32,
}

enum IdKind {
    Pt,
    Cd,
}

// this tracks multiple regions
pub struct RegionUniverse {
    conf: RegionUniverseConf,
    pt_map: HashMap<String, Vec2>,
    shape_map: HashMap<String, CurveDrawer>,
    mp_map: HashMap<RegionName, MultiPolygon>,
}
impl RegionUniverse {
    pub fn new(conf: RegionUniverseConf, shapes: &[(String, CurveDrawer)]) -> LivecodeResult<Self> {
        let tolerance = conf.tolerance;
        let mut shape_map = HashMap::new();
        let mut mp_map = HashMap::new();
        let pt_map = HashMap::new();

        for (n, cd) in shapes.iter() {
            shape_map.insert(n.clone(), cd.clone()); // we'll keep this around cuz sometimes it's nice to query it

            if cd.is_closed() {
                let mp = line_to_multipolygon(&cd.flatten_with_lyon(tolerance)?);
                mp_map.insert(RegionName::new(n), mp);
            }
        }

        Ok(RegionUniverse {
            conf,
            shape_map,
            mp_map,
            pt_map,
        })
    }

    pub fn query_specs(
        &mut self,
        specs: &[RegionSpec],
    ) -> LivecodeResult<Vec<(String, Vec<CurveDrawer>)>> {
        let mut out = Vec::with_capacity(specs.len());
        for spec in specs {
            let result = spec.eval(&self)?;
            self.add_region(RegionName::new(&spec.name), result.clone());
            let cds = mp_to_cds(&result, self.conf.tolerance);
            out.push((spec.name.clone(), cds));
        }
        Ok(out)
    }

    fn lookup_many(
        &self,
        members: &RegionList,
        shape_scope: &ShapeScope,
    ) -> LivecodeResult<Vec<&MultiPolygon>> {
        let mut operands = Vec::new();
        for m in members.regions() {
            if let Ok(Some(mp)) = self.lookup(m, shape_scope) {
                operands.push(mp);
            }
        }
        Ok(operands)
    }

    fn lookup(
        &self,
        m: &RegionName,
        shape_scope: &ShapeScope,
    ) -> LivecodeResult<Option<&MultiPolygon>> {
        if shape_scope.allows_for(m) {
            self.mp_map
                .get(m)
                .map(|x| Some(x))
                .ok_or_else(|| "missing")
                .to_lc_err()
        } else {
            // not an error, but excluded
            Ok(None)
        }
    }

    fn lookup_point(&self, v: &str) -> Option<&Vec2> {
        self.pt_map.get(v)
    }

    fn get_in_out(&self, pt: Point) -> (RegionList, RegionList) {
        let mut ins = Vec::new();
        let mut outs = Vec::new();

        for (name, mp) in self.mp_map.iter() {
            // we know this is a cd

            if (mp.unsigned_area() as f32) < MIN_UNIVERSE_AREA {
                continue;
            }
            if mp.contains(&pt) {
                ins.push(name.clone());
            } else {
                outs.push(name.clone());
            }
        }

        (RegionList(ins), RegionList(outs))
    }

    fn add_region(&mut self, name: RegionName, mp: MultiPolygon) {
        self.mp_map.insert(name, mp);
    }
}
