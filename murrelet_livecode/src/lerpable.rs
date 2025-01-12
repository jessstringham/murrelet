use std::collections::HashMap;

use evalexpr::Node;
use glam::{vec2, vec3, Vec2, Vec3};
use murrelet_common::{lerp, MurreletColor};

use crate::{lazy::LazyNodeF32, types::AdditionalContextNode};

pub trait Lerpable {
    fn lerp(&self, other: &Self, pct: f32) -> Self;
}

impl Lerpable for f32 {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        lerp(*self, *other, pct)
    }
}

impl Lerpable for u64 {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as u64
    }
}

impl Lerpable for u8 {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as u8
    }
}

impl Lerpable for usize {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as usize
    }
}

impl Lerpable for i32 {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        lerp(*self as f32, *other as f32, pct) as i32
    }
}

impl Lerpable for Vec2 {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        vec2(lerp(self.x, other.x, pct), lerp(self.y, other.y, pct))
    }
}

impl Lerpable for Vec3 {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        vec3(
            lerp(self.x, other.x, pct),
            lerp(self.y, other.y, pct),
            lerp(self.z, other.z, pct),
        )
    }
}

impl Lerpable for MurreletColor {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
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

impl Lerpable for bool {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        if pct > 0.5 {
            *other
        } else {
            *self
        }
    }
}

impl Lerpable for String {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        if pct > 0.5 {
            other.clone()
        } else {
            self.clone()
        }
    }
}

// i think this shouldn't matter....
impl Lerpable for AdditionalContextNode {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        if pct > 0.5 {
            other.clone()
        } else {
            self.clone()
        }
    }
}

// same??
impl Lerpable for Node {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        if pct > 0.5 {
            other.clone()
        } else {
            self.clone()
        }
    }
}

// not sure about this either...
impl Lerpable for LazyNodeF32 {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        if pct > 0.5 {
            other.clone()
        } else {
            self.clone()
        }
    }
}

impl<K: Clone, V: Clone> Lerpable for HashMap<K, V> {
    fn lerp(&self, other: &Self, pct: f32) -> Self {
        if pct > 0.5 {
            other.clone()
        } else {
            self.clone()
        }
    }
}
