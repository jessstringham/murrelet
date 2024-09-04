// these could probably be somewhere else, but they are used by this crate
// and just give more livecode defaults to work with instead of always
// needing to create a new type.
use glam::{Vec2, Vec3};
use murrelet_common::MurreletColor;
use murrelet_livecode_derive::{Livecode, UnitCell};

#[derive(Copy, Clone, Debug, Livecode, Default)]
pub struct F32Newtype {
    v: f32,
}

impl F32Newtype {
    pub fn v(&self) -> f32 {
        self.v
    }
}

#[derive(Copy, Clone, Debug, Livecode, Default)]
pub struct Vec2Newtype {
    v: Vec2,
}

impl Vec2Newtype {
    pub fn new(v: Vec2) -> Self {
        Self { v }
    }

    pub fn vec2(&self) -> Vec2 {
        self.v
    }
}

#[derive(Copy, Clone, Debug, Livecode, Default)]
pub struct Vec3Newtype {
    v: Vec3,
}

impl Vec3Newtype {
    pub fn new(v: Vec3) -> Self {
        Self { v }
    }

    pub fn vec3(&self) -> Vec3 {
        self.v
    }
}

#[derive(Copy, Clone, Debug, Livecode, Default)]
pub struct RGBandANewtype {
    rgb: Vec3,
    a: f32,
}

impl RGBandANewtype {
    pub fn new(rgb: Vec3, a: f32) -> Self {
        Self { rgb, a }
    }

    pub fn rgba(&self) -> [f32; 4] {
        [self.rgb.x, self.rgb.y, self.rgb.z, self.a]
    }

    pub fn color(&self) -> MurreletColor {
        let [r, g, b, a] = self.rgba();
        MurreletColor::rgba(r, g, b, a)
    }

    pub fn with_alpha(&self, alpha: f32) -> Self {
        let mut c = self.clone();
        c.a = alpha;
        c
    }
}
