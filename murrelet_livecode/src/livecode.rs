#![allow(dead_code)]
use std::collections::HashMap;
use std::collections::HashSet;

use evalexpr::build_operator_tree;
use evalexpr::Node;
use glam::vec2;
use glam::vec3;
use glam::Vec2;
use glam::Vec3;
use itertools::Itertools;
use murrelet_common::clamp;

use murrelet_common::MurreletColor;
use serde::Deserialize;

use crate::lazy::ControlLazyNodeF32;
use crate::lazy::LazyNodeF32;
use crate::state::LivecodeWorldState;
use crate::types::AdditionalContextNode;
use crate::types::ControlVecElement;
use crate::types::LivecodeError;
use crate::types::LivecodeResult;

// for default values
pub fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

pub trait LivecodeFromWorld<T> {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<T>;
}

impl LivecodeFromWorld<f32> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<f32> {
        self._o(w)
    }
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

impl LivecodeToControl<ControlLazyNodeF32> for LazyNodeF32 {
    fn to_control(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::new(self.n().cloned().unwrap())
    }
}

// wrappers around identifiers evalexpr gives us, right now
// just to control midi controller
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct LivecodeVariable {
    pub name: String,
}
impl LivecodeVariable {
    pub fn from_str(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct LivecodeFunction {
    name: String,
}
impl LivecodeFunction {
    pub fn from_str(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

pub trait GetLivecodeIdentifiers {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable>;
    fn function_identifiers(&self) -> Vec<LivecodeFunction>;
}

impl GetLivecodeIdentifiers for ControlF32 {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        match self {
            ControlF32::Int(_) => vec![],
            ControlF32::Bool(_) => vec![],
            ControlF32::Float(_) => vec![],
            ControlF32::Expr(node) => node
                .iter_variable_identifiers()
                .dedup()
                .map(|x| LivecodeVariable::from_str(x))
                .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        match self {
            ControlF32::Int(_) => vec![],
            ControlF32::Bool(_) => vec![],
            ControlF32::Float(_) => vec![],
            ControlF32::Expr(node) => node
                .iter_variable_identifiers()
                .dedup()
                .map(|x| LivecodeFunction::from_str(x))
                .collect_vec(),
        }
    }
}

impl GetLivecodeIdentifiers for [ControlF32; 2] {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        vec![
            self[0].variable_identifiers(),
            self[1].variable_identifiers(),
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<LivecodeVariable>>()
        .into_iter()
        .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        vec![
            self[0].function_identifiers(),
            self[1].function_identifiers(),
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<LivecodeFunction>>()
        .into_iter()
        .collect_vec()
    }
}

impl GetLivecodeIdentifiers for [ControlF32; 3] {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        vec![
            self[0].variable_identifiers(),
            self[1].variable_identifiers(),
            self[2].variable_identifiers(),
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<LivecodeVariable>>()
        .into_iter()
        .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        vec![
            self[0].function_identifiers(),
            self[1].function_identifiers(),
            self[2].function_identifiers(),
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<LivecodeFunction>>()
        .into_iter()
        .collect_vec()
    }
}

impl GetLivecodeIdentifiers for [ControlF32; 4] {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        vec![
            self[0].variable_identifiers(),
            self[1].variable_identifiers(),
            self[2].variable_identifiers(),
            self[3].variable_identifiers(),
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<LivecodeVariable>>()
        .into_iter()
        .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        vec![
            self[0].function_identifiers(),
            self[1].function_identifiers(),
            self[2].function_identifiers(),
            self[3].function_identifiers(),
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<LivecodeFunction>>()
        .into_iter()
        .collect_vec()
    }
}

impl GetLivecodeIdentifiers for String {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        vec![]
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        vec![]
    }
}

impl GetLivecodeIdentifiers for AdditionalContextNode {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        vec![]
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        vec![]
    }
}

impl<K, V> GetLivecodeIdentifiers for HashMap<K, V> {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        vec![]
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        vec![]
    }
}

impl GetLivecodeIdentifiers for ControlBool {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        match self {
            ControlBool::Int(_) => vec![],
            ControlBool::Raw(_) => vec![],
            ControlBool::Float(_) => vec![],
            ControlBool::Expr(node) => node
                .iter_variable_identifiers()
                .dedup()
                .map(|x| LivecodeVariable::from_str(x))
                .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        match self {
            ControlBool::Int(_) => vec![],
            ControlBool::Raw(_) => vec![],
            ControlBool::Float(_) => vec![],
            ControlBool::Expr(node) => node
                .iter_variable_identifiers()
                .dedup()
                .map(|x| LivecodeFunction::from_str(x))
                .collect_vec(),
        }
    }
}

pub fn empty_string() -> String {
    String::new()
}

pub fn empty_string_lazy() -> String {
    String::new()
}

pub fn _auto_default_f32_0_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(0.0)
}
pub fn _auto_default_f32_1_lazy() -> ControlLazyNodeF32 {
    ControlLazyNodeF32::Float(1.0)
}

// this is to handle the Vec<Lazy> ones, which goes up to length 4 for color
// and doesn't care if there are too many
pub fn _auto_default_f32_vec0_lazy() -> Vec<ControlVecElement<ControlLazyNodeF32>> {
    vec![
        ControlVecElement::raw(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(0.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(0.0)),
    ]
}
pub fn _auto_default_f32_vec1_lazy() -> Vec<ControlVecElement<ControlLazyNodeF32>> {
    vec![
        ControlVecElement::raw(ControlLazyNodeF32::Float(1.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(1.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(1.0)),
        ControlVecElement::raw(ControlLazyNodeF32::Float(1.0)),
    ]
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

pub fn _auto_default_vec2_0_lazy() -> [ControlLazyNodeF32; 2] {
    [
        ControlLazyNodeF32::Float(0.0),
        ControlLazyNodeF32::Float(0.0),
    ]
}
pub fn _auto_default_vec2_1_lazy() -> [ControlLazyNodeF32; 2] {
    [
        ControlLazyNodeF32::Float(1.0),
        ControlLazyNodeF32::Float(1.0),
    ]
}

pub fn _auto_default_color_0() -> [ControlF32; 4] {
    [
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
    ]
}
pub fn _auto_default_color_1() -> [ControlF32; 4] {
    [
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
    ]
}

pub fn _auto_default_bool_false() -> ControlBool {
    ControlBool::Raw(false)
}
pub fn _auto_default_bool_true() -> ControlBool {
    ControlBool::Raw(true)
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
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

    pub fn _o(&self, w: &LivecodeWorldState) -> LivecodeResult<f32> {
        match self {
            ControlF32::Bool(b) => {
                if *b {
                    Ok(1.0)
                } else {
                    Ok(-1.0)
                }
            }
            ControlF32::Int(i) => Ok(*i as f32),
            ControlF32::Float(x) => Ok(*x),
            ControlF32::Expr(e) => match e.eval_float_with_context(w.ctx()).map(|x| x as f32) {
                Ok(r) => Ok(r),
                Err(_) => {
                    let b = e
                        .eval_boolean_with_context(w.ctx())
                        .map_err(|err| LivecodeError::EvalExpr(format!("evalexpr err"), err));
                    Ok(if b? { 1.0 } else { -1.0 })
                }
            },
        }
    }
}

impl Default for ControlBool {
    fn default() -> Self {
        Self::Raw(true)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlBool {
    Raw(bool),
    Int(i32),
    Float(f32),
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Expr(Node),
}
impl ControlBool {
    // pub fn to_unitcell_control(&self) -> UnitCellControlExprBool {
    //     match self {
    //         ControlBool::Raw(x) => UnitCellControlExprBool::Bool(*x),
    //         ControlBool::Int(x) => UnitCellControlExprBool::Int(*x),
    //         ControlBool::Float(x) => UnitCellControlExprBool::Float(*x),
    //         ControlBool::Expr(x) => UnitCellControlExprBool::Expr(x.clone()),
    //     }
    // }

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
        // self.to_unitcell_control().eval(w)

        match self {
            ControlBool::Raw(b) => Ok(*b),
            ControlBool::Int(i) => Ok(*i > 0),
            ControlBool::Float(x) => Ok(*x > 0.0),
            ControlBool::Expr(e) => match e.eval_boolean_with_context(w.ctx()) {
                Ok(r) => Ok(r),
                Err(_) => {
                    let b = e.eval_float_with_context(w.ctx()).map_err(|err| {
                        LivecodeError::EvalExpr(format!("error evaluing bool"), err)
                    });
                    b.map(|x| x > 0.0)
                }
            },
        }
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
