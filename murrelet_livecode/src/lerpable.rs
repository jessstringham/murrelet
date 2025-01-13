use std::collections::HashMap;

use evalexpr::Node;
use glam::{vec2, vec3, Vec2, Vec3};
use itertools::Itertools;
use murrelet_common::{lerp, MurreletColor};

use crate::{lazy::LazyNodeF32, types::AdditionalContextNode, unitcells::{UnitCell, UnitCellContext}};

pub fn step<T: Clone>(this: &T, other: &T, pct: f32) -> T {
    if pct > 0.5 {
        other.clone()
    } else {
        this.clone()
    }
}

pub fn combine_vecs<T: Clone + Lerpable>(this: &Vec<T>, other: &Vec<T>, pct: f32) -> Vec<T> {
    // for now, just take the shortest, but we'll update this...
    this.iter()
        .zip(other.iter())
        .map(|(a, b)| a.lerpify(b, pct))
        .collect_vec()
}

pub trait Lerpable {
    // sorry, making it a unique name...
    fn lerpify(&self, other: &Self, pct: f32) -> Self;
}

impl Lerpable for f32 {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        lerp(*self, *other, pct)
    }
}

impl Lerpable for u64 {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as u64
    }
}

impl Lerpable for u8 {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as u8
    }
}

impl Lerpable for usize {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as usize
    }
}

impl Lerpable for i32 {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as i32
    }
}

impl Lerpable for Vec2 {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        vec2(lerp(self.x, other.x, pct), lerp(self.y, other.y, pct))
    }
}

impl Lerpable for Vec3 {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        vec3(
            lerp(self.x, other.x, pct),
            lerp(self.y, other.y, pct),
            lerp(self.z, other.z, pct),
        )
    }
}

impl Lerpable for MurreletColor {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        let [h, s, v, a] = self.into_hsva_components();
        let [h2, s2, v2, a2] = other.into_hsva_components();
        MurreletColor::hsva(
            lerp(h, h2, pct),
            lerp(s, s2, pct),
            lerp(v, v2, pct),
            lerp(a, a2, pct),
        )
    }
}



impl Lerpable for UnitCellContext {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {

        let ctx = self.ctx().experimental_lerp(&other.ctx(), pct);
        let detail = self.detail.experimental_lerp(&other.detail, pct);
        let tile_info = step(&self.tile_info, &other.tile_info, pct);
        UnitCellContext::new_with_option_info(ctx, detail, tile_info)

    }
}

impl<T: Lerpable> Lerpable for UnitCell<T> {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        let node = self.node.lerpify(&other.node, pct);


        let detail = self.detail.lerpify(&other.detail, pct);

        UnitCell::new(node, detail)
    }
}

impl Lerpable for bool {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        step(self, other, pct)
    }
}

impl Lerpable for String {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        step(self, other, pct)
    }
}

// i think this shouldn't matter....
impl Lerpable for AdditionalContextNode {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        step(self, other, pct)
    }
}

// same??
impl Lerpable for Node {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        step(self, other, pct)
    }
}

// not sure about this either...
impl Lerpable for LazyNodeF32 {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        step(self, other, pct)
    }
}

impl<K: Clone, V: Clone> Lerpable for HashMap<K, V> {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        step(self, other, pct)
    }
}
