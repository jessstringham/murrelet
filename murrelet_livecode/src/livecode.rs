#![allow(dead_code)]
use evalexpr::build_operator_tree;
use evalexpr::Node;
use glam::vec2;
use glam::vec3;
use glam::Vec2;
use glam::Vec3;
use murrelet_common::clamp;

use murrelet_common::MurreletColor;
use serde::Deserialize;

use crate::state::LivecodeWorldState;
use crate::types::LazyNodeF32;
use crate::types::LazyNodeF32Def;
use crate::types::LivecodeResult;
use crate::unitcells::{EvaluableUnitCell, UnitCellControlExprBool, UnitCellControlExprF32};

// for default values
pub fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

pub trait LivecodeFromWorld<T> {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<T>;
}

impl LivecodeFromWorld<Vec2> for [ControlF32; 2] {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec2> {
        Ok(vec2(self[0].o(w)?, self[1].o(w)?))
    }
}

impl LivecodeFromWorld<Vec3> for [ControlF32; 3] {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec3> {
        Ok(vec3(self[0].o(w)?, self[1].o(w)?, self[2].o(w)?))
    }
}

impl LivecodeFromWorld<MurreletColor> for [ControlF32; 4] {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<MurreletColor> {
        // by default, clamp saturation and value
        Ok(MurreletColor::hsva(
            self[0].o(w)?,
            clamp(self[1].o(w)?, 0.0, 1.0),
            clamp(self[2].o(w)?, 0.0, 1.0),
            self[3].o(w)?,
        ))
    }
}

pub trait LivecodeToControl<ControlT> {
    fn to_control(&self) -> ControlT;
}

impl LivecodeToControl<ControlF32> for f32 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self)
    }
}

impl LivecodeToControl<ControlF32> for i32 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlF32> for u32 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlF32> for u8 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlBool> for bool {
    fn to_control(&self) -> ControlBool {
        ControlBool::Raw(*self)
    }
}

impl LivecodeToControl<[ControlF32; 2]> for Vec2 {
    fn to_control(&self) -> [ControlF32; 2] {
        [self.x.to_control(), self.y.to_control()]
    }
}

impl LivecodeToControl<[ControlF32; 3]> for Vec3 {
    fn to_control(&self) -> [ControlF32; 3] {
        [
            self.x.to_control(),
            self.y.to_control(),
            self.z.to_control(),
        ]
    }
}

impl LivecodeToControl<[ControlF32; 4]> for MurreletColor {
    fn to_control(&self) -> [ControlF32; 4] {
        let [r, g, b, a] = self.into_rgba_components();
        [
            r.to_control(),
            g.to_control(),
            b.to_control(),
            a.to_control(),
        ]
    }
}

impl LivecodeToControl<ControlF32> for usize {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<ControlF32> for u64 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl LivecodeToControl<LazyNodeF32Def> for LazyNodeF32 {
    fn to_control(&self) -> LazyNodeF32Def {
        LazyNodeF32Def::new(self.n().cloned().unwrap())
    }
}

// i don't know if this is a good place to put this...
pub fn _auto_default_f32_0() -> ControlF32 {
    ControlF32::Raw(0.0)
}
pub fn _auto_default_f32_1() -> ControlF32 {
    ControlF32::Raw(1.0)
}

pub fn _auto_default_vec2_0() -> [ControlF32; 2] {
    [ControlF32::Raw(0.0), ControlF32::Raw(0.0)]
}
pub fn _auto_default_vec2_1() -> [ControlF32; 2] {
    [ControlF32::Raw(1.0), ControlF32::Raw(1.0)]
}

pub fn _auto_default_bool_false() -> ControlBool {
    ControlBool::Raw(false)
}
pub fn _auto_default_bool_true() -> ControlBool {
    ControlBool::Raw(true)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ControlF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    Expr(Node),
}

impl ControlF32 {
    // for backwards compatibility
    #[allow(non_snake_case)]
    pub fn Raw(v: f32) -> ControlF32 {
        Self::Float(v)
    }

    pub fn force_from_str(s: &str) -> ControlF32 {
        match build_operator_tree(s) {
            Ok(e) => Self::Expr(e),
            Err(err) => {
                println!("{:?}", err);
                ControlF32::Raw(1.0)
            }
        }
    }

    pub fn to_unitcell_control(&self) -> UnitCellControlExprF32 {
        match self {
            ControlF32::Int(x) => UnitCellControlExprF32::Int(*x),
            ControlF32::Bool(x) => UnitCellControlExprF32::Bool(*x),
            ControlF32::Float(x) => UnitCellControlExprF32::Float(*x),
            ControlF32::Expr(x) => UnitCellControlExprF32::Expr(x.clone()),
        }
    }

    pub fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<f32> {
        self.to_unitcell_control().eval(&w)
    }
}

impl Default for ControlBool {
    fn default() -> Self {
        Self::Raw(true)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ControlBool {
    Raw(bool),
    Int(i32),
    Float(f32),
    Expr(Node),
}
impl ControlBool {
    pub fn to_unitcell_control(&self) -> UnitCellControlExprBool {
        match self {
            ControlBool::Raw(x) => UnitCellControlExprBool::Bool(*x),
            ControlBool::Int(x) => UnitCellControlExprBool::Int(*x),
            ControlBool::Float(x) => UnitCellControlExprBool::Float(*x),
            ControlBool::Expr(x) => UnitCellControlExprBool::Expr(x.clone()),
        }
    }

    pub fn force_from_str(s: &str) -> ControlBool {
        match build_operator_tree(s) {
            Ok(e) => Self::Expr(e),
            Err(err) => {
                println!("{:?}", err);
                ControlBool::Raw(false)
            }
        }
    }

    pub fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<bool> {
        self.to_unitcell_control().eval(w)
    }

    pub fn default(&self) -> bool {
        match self {
            ControlBool::Raw(x) => *x,
            ControlBool::Int(x) => *x > 0,
            ControlBool::Float(x) => *x > 0.0,
            ControlBool::Expr(_) => false, // eh
        }
    }
}
