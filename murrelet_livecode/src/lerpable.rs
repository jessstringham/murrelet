use std::{collections::HashMap, fmt::Debug};

use evalexpr::Node;
use glam::{vec2, vec3, Vec2, Vec3};
use murrelet_common::{lerp, MurreletColor};

use crate::{
    lazy::LazyNodeF32,
    types::AdditionalContextNode,
    unitcells::{UnitCell, UnitCellContext},
};

pub fn step<T: Clone>(this: &T, other: &T, pct: f32) -> T {
    if pct > 0.5 {
        other.clone()
    } else {
        this.clone()
    }
}

pub fn combine_vecs<T: Clone + Lerpable + Debug>(
    this: &Vec<T>,
    other: &Vec<T>,
    pct: f32,
) -> Vec<T> {
    // for now, just take the shortest, but we'll update this...

    let mut v = vec![];
    // figure out how many to show
    let this_len = this.len();
    let other_len = other.len();
    // round is important! or can get cases where two things of the same length return a count of something less!
    // could also do a special check, but i think this might look better too?
    let count = if this_len == other_len {
        this_len
    } else {
        lerp(this_len as f32, other_len as f32, pct).round() as usize
    };
    // println!("start");
    // println!("pct {} this_len {} other_len {} count {}", pct, this_len, other_len, count);
    for i in 0..count {
        // println!("sss i {:?} this has (i) {:?} that has (i) {:?}", i, this.get(i).is_some(), other.get(i).is_some());
        // println!("sss i {:?} this has (i) {:?} that has (i) {:?}", i, this.get(i), other.get(i));
        let result = match (i >= this_len, i >= other_len) {
            (true, true) => unreachable!(),
            (true, false) => other[i].clone(),
            (false, true) => this[i].clone(),
            (false, false) => this[i].lerpify(&other[i], pct),
        };
        v.push(result);
    }
    v

    // this.iter()
    //     .zip(other.iter())
    //     .map(|(a, b)| a.lerpify(b, pct))
    //     .collect_vec()
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

impl<T: Lerpable + Clone + Debug> Lerpable for Vec<T> {
    fn lerpify(&self, other: &Self, pct: f32) -> Self {
        if self.len() == 0 || other.len() == 0 {
            return self.clone();
        }
        combine_vecs(&self, &other, pct)
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
