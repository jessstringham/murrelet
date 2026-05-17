//! `Polyline` (alias for `Vec<Vec2>`), `AngledPolyline`, and their conversion traits.
use glam::Vec2;
use itertools::Itertools;

use crate::geometry::SpotOnCurve;

pub type Polyline = Vec<Vec2>;

// IsPolyline traits

pub trait IsPolyline {
    fn into_iter_vec2<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = Vec2> + 'a>;
    fn as_polyline(self) -> Polyline;
    fn into_vec(self) -> Vec<Vec2>;
    fn clone_to_vec(&self) -> Vec<Vec2>;
}

impl IsPolyline for &[Vec2] {
    fn into_iter_vec2<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = Vec2> + 'a> {
        Box::new(self.iter().cloned())
    }

    fn as_polyline(self) -> Polyline {
        self.to_vec()
    }

    fn into_vec(self) -> Vec<Vec2> {
        self.to_vec()
    }

    fn clone_to_vec(&self) -> Vec<Vec2> {
        self.to_vec()
    }
}

impl IsPolyline for Vec<Vec2> {
    fn into_iter_vec2<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = Vec2> + 'a> {
        Box::new(self.iter().cloned())
    }

    fn as_polyline(self) -> Polyline {
        self
    }

    fn into_vec(self) -> Vec<Vec2> {
        self
    }

    fn clone_to_vec(&self) -> Vec<Vec2> {
        self.clone()
    }
}

// ANGLED POLYLINE
// like polyline, but uses SpotOnCurve

#[derive(Debug, Clone)]
pub struct AngledPolyline {
    v: Vec<SpotOnCurve>,
}

impl AngledPolyline {
    pub fn new(v: Vec<SpotOnCurve>) -> Self {
        Self { v }
    }

    pub fn empty() -> Self {
        Self { v: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.v.is_empty()
    }

    pub fn reverse(&mut self) {
        self.v.reverse()
    }

    pub fn first(&self) -> Option<&SpotOnCurve> {
        self.v.first()
    }

    pub fn last(&self) -> Option<&SpotOnCurve> {
        self.v.last()
    }

    pub fn len(&self) -> usize {
        self.v.len()
    }

    pub fn vertices(&self) -> &[SpotOnCurve] {
        &self.v
    }
}

impl IsPolyline for AngledPolyline {
    fn into_iter_vec2<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = Vec2> + 'a> {
        Box::new(self.v.iter().map(|x| x.loc()))
    }

    fn as_polyline(self) -> Polyline {
        // actually cloning...
        self.into_iter_vec2().collect_vec().as_polyline()
    }

    // actually cloning...
    fn into_vec(self) -> Vec<Vec2> {
        self.into_iter_vec2().collect_vec()
    }

    fn clone_to_vec(&self) -> Vec<Vec2> {
        self.into_iter_vec2().collect_vec()
    }
}

pub trait IsAngledPolyline {
    fn into_iter_spot<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = SpotOnCurve> + 'a>;
    fn as_angled_polyline(self) -> AngledPolyline;
    fn into_vec(self) -> Vec<SpotOnCurve>;
    fn clone_to_spot_vec(&self) -> Vec<SpotOnCurve> {
        self.into_iter_spot().collect_vec()
    }
}

impl IsAngledPolyline for &[SpotOnCurve] {
    fn into_iter_spot<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = SpotOnCurve> + 'a> {
        Box::new(self.iter().cloned())
    }

    fn as_angled_polyline(self) -> AngledPolyline {
        AngledPolyline { v: self.to_vec() }
    }

    fn into_vec(self) -> Vec<SpotOnCurve> {
        self.to_vec()
    }
}

impl IsAngledPolyline for Vec<SpotOnCurve> {
    fn into_iter_spot<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = SpotOnCurve> + 'a> {
        Box::new(self.iter().cloned())
    }

    fn as_angled_polyline(self) -> AngledPolyline {
        AngledPolyline { v: self.to_vec() }
    }

    fn into_vec(self) -> Vec<SpotOnCurve> {
        self.to_vec()
    }
}

impl IsAngledPolyline for AngledPolyline {
    fn into_iter_spot<'a>(&'a self) -> Box<dyn ExactSizeIterator<Item = SpotOnCurve> + 'a> {
        Box::new(self.v.iter().cloned())
    }

    fn as_angled_polyline(self) -> AngledPolyline {
        self
    }

    fn into_vec(self) -> Vec<SpotOnCurve> {
        self.v
    }
}
