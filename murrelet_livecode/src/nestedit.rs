// Used by thte NestedIt, to make it possible to read/edit the values of a struct
// using a dot delimited string. I use this for the

use std::collections::HashMap;

use evalexpr::Node;
use glam::{vec2, vec3, Vec2, Vec3};
use murrelet_common::MurreletColor;

use crate::unitcells::UnitCellCtx;

#[derive(Debug, Clone)]
pub struct NestedMod<'a> {
    curr_loc: Vec<String>,
    mods: &'a HashMap<String, String>, // modify strings
}

impl<'a> NestedMod<'a> {
    pub fn from_dict(mods: &'a HashMap<String, String>) -> Self {
        Self {
            curr_loc: vec![],
            mods,
        }
    }

    pub fn get_curr(&self) -> Option<String> {
        self.mods.get(&self.curr_loc.join(".")).cloned()
    }

    pub fn get_subfield(&self, subfield: &str) -> Option<String> {
        let mut keys = self.curr_loc.clone();
        keys.push(subfield.to_owned());
        self.mods.get(&keys.join(".")).cloned()
    }

    pub fn get_subfield_as_f32(&self, subfield: &str) -> Option<f32> {
        if let Some(s) = self.get_subfield(subfield) {
            s.parse::<f32>().ok()
        } else {
            None
        }
    }

    pub fn get_curr_as_f32(&self) -> Option<f32> {
        if let Some(s) = self.get_curr() {
            s.parse::<f32>().ok()
        } else {
            None
        }
    }

    pub fn next_loc(&self, subfield: &str) -> Self {
        let mut curr_loc = self.curr_loc.clone();
        curr_loc.push(subfield.to_owned());

        Self {
            curr_loc,
            mods: self.mods,
        }
    }
}

pub trait NestEditable
where
    Self: Sized,
{
    fn nest_update(&self, mods: NestedMod) -> Self;
}

impl NestEditable for f32 {
    fn nest_update(&self, mods: NestedMod) -> Self {
        mods.get_curr_as_f32().unwrap_or(*self)
    }
}

impl NestEditable for u64 {
    fn nest_update(&self, mods: NestedMod) -> Self {
        mods.get_curr_as_f32().map(|x| x as u64).unwrap_or(*self)
    }
}

impl NestEditable for u8 {
    fn nest_update(&self, mods: NestedMod) -> Self {
        mods.get_curr_as_f32().map(|x| x as u8).unwrap_or(*self)
    }
}

impl NestEditable for usize {
    fn nest_update(&self, mods: NestedMod) -> Self {
        mods.get_curr_as_f32().map(|x| x as usize).unwrap_or(*self)
    }
}

impl NestEditable for i32 {
    fn nest_update(&self, mods: NestedMod) -> Self {
        mods.get_curr_as_f32().map(|x| x as i32).unwrap_or(*self)
    }
}

impl NestEditable for Vec2 {
    fn nest_update(&self, mods: NestedMod) -> Self {
        let x = mods.get_subfield_as_f32("x").unwrap_or(self.x);
        let y = mods.get_subfield_as_f32("y").unwrap_or(self.y);

        vec2(x, y)
    }
}

impl NestEditable for Vec3 {
    fn nest_update(&self, mods: NestedMod) -> Self {
        let x = mods.get_subfield_as_f32("x").unwrap_or(self.x);
        let y = mods.get_subfield_as_f32("y").unwrap_or(self.y);
        let z = mods.get_subfield_as_f32("z").unwrap_or(self.z);

        vec3(x, y, z)
    }
}

impl NestEditable for MurreletColor {
    fn nest_update(&self, mods: NestedMod) -> Self {
        let maybe_h = mods.get_subfield_as_f32("h");
        let maybe_s = mods.get_subfield_as_f32("s");
        let maybe_v = mods.get_subfield_as_f32("v");
        let maybe_a = mods.get_subfield_as_f32("a");

        if let (Some(h), Some(s), Some(v), Some(a)) = (maybe_h, maybe_s, maybe_v, maybe_a) {
            MurreletColor::hsva(h, s, v, a)
        } else {
            self.clone()
        }
    }
}

impl NestEditable for bool {
    fn nest_update(&self, mods: NestedMod) -> Self {
        if let Some(x) = mods.get_curr() {
            match x.as_str() {
                "true" => true,
                "false" => false,
                _ => *self,
            }
        } else {
            *self
        }
    }
}

impl NestEditable for String {
    fn nest_update(&self, mods: NestedMod) -> Self {
        mods.get_curr().unwrap_or(self.clone())
    }
}

impl NestEditable for UnitCellCtx {
    fn nest_update(&self, _mods: NestedMod) -> Self {
        self.clone() // noop
    }
}

impl NestEditable for Node {
    fn nest_update(&self, _mods: NestedMod) -> Self {
        self.clone() // noop
    }
}

impl<K: Clone, V: Clone> NestEditable for HashMap<K, V> {
    fn nest_update(&self, _mods: NestedMod) -> Self {
        self.clone() // noop
    }
}
