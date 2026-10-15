#![allow(dead_code)]
use crate::expr::expr_as_vec4;
use crate::lazy::ControlLazyNodeF32;
use crate::lazy::LazyNodeF32;
use crate::state::LivecodeWorldState;
use crate::types::AdditionalContextNode;
use crate::types::ControlVecElement;
use evalexpr::Node;
use evalexpr::build_operator_tree;
use glam::Vec2;
use glam::Vec3;
use glam::vec2;
use glam::vec3;
use itertools::Itertools;
use murrelet_common::AnglePi;
use murrelet_common::LivecodeError;
use murrelet_common::LivecodeResult;
use murrelet_common::MurreletColor;
use murrelet_common::MurreletString;
use murrelet_common::ToLivecodeResult;
use murrelet_common::clamp;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;

#[doc(hidden)]
pub use paste;

//claude

// Newtype that carries an evalexpr `Node` alongside the source string it was
// parsed from, so the Control config can round-trip through serde (the bare
// `Node`'s `Display` is lossy prefix notation that won't re-parse).
#[derive(Debug, Clone)]
pub struct ExprNode {
    src: String,
    node: Node,
}

impl ExprNode {
    // pub fn from_src(src: String) -> LivecodeResult<Self> {
    //     let node = build_operator_tree(&src)
    //         .map_err(|err| LivecodeError::EvalExpr("error parsing expr".to_string(), err))?;
    //     Ok(Self { src, node })
    // }

    pub fn from_src(src: String) -> LivecodeResult<Self> {
        let to_parse = desugar_expr_src(&src)?.unwrap_or_else(|| src.clone());
        let node = build_operator_tree(&to_parse).map_err(|err| {
            LivecodeError::EvalExpr("error parsing expr".to_string(), err.to_string())
        })?;
        Ok(Self { src, node })
    }

    // fallback for Node-only construction (src is lossy Display — only for
    // non-config paths)
    pub fn from_node(node: Node) -> Self {
        Self {
            src: node.to_string(),
            node,
        }
    }

    pub fn node(&self) -> &Node {
        &self.node
    }
}

impl std::ops::Deref for ExprNode {
    type Target = Node;
    fn deref(&self) -> &Node {
        &self.node
    }
}

impl Serialize for ExprNode {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.serialize_str(&self.src)
    }
}

impl<'de> Deserialize<'de> for ExprNode {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // let s = String::deserialize(d)?;
        // let node = build_operator_tree(&s).map_err(serde::de::Error::custom)?;
        // Ok(ExprNode { src: s, node })

        let s = String::deserialize(d)?;
        ExprNode::from_src(s).map_err(serde::de::Error::custom)
    }
}

// for default values
pub fn empty_vec<T>() -> Vec<T> {
    Vec::new()
}

pub trait LivecodeFromWorld<T> {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<T>;

    fn o_dummy(&self) -> LivecodeResult<T> {
        self.o(&LivecodeWorldState::new_dummy())
    }
}

impl LivecodeFromWorld<f32> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<f32> {
        self._o(w)
    }
}

impl LivecodeFromWorld<f64> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<f64> {
        Ok(self._o(w)? as f64)
    }
}

impl LivecodeFromWorld<usize> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<usize> {
        Ok(self._o(w)? as usize)
    }
}

impl LivecodeFromWorld<u8> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<u8> {
        Ok(self._o(w)? as u8)
    }
}

impl LivecodeFromWorld<u32> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<u32> {
        Ok(self._o(w)? as u32)
    }
}

impl LivecodeFromWorld<u64> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<u64> {
        Ok(self._o(w)? as u64)
    }
}

impl LivecodeFromWorld<i32> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<i32> {
        Ok(self._o(w)? as i32)
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
        // by default, clamp saturation and value to save some headaches later
        Ok(MurreletColor::hsva(
            self[0].o(w)?,
            clamp(self[1].o(w)?, 0.0, 1.0),
            clamp(self[2].o(w)?, 0.0, 1.0),
            self[3].o(w)?,
        ))
    }
}

impl LivecodeFromWorld<AnglePi> for ControlF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<AnglePi> {
        Ok(AnglePi::new(self._o(w)?))
    }
}

impl<Source, Target> LivecodeFromWorld<Vec<Target>> for Vec<Source>
where
    Source: LivecodeFromWorld<Target>,
{
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>> {
        self.iter().map(|x| x.o(w)).collect::<Result<Vec<_>, _>>()
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

impl LivecodeToControl<ControlF32> for AnglePi {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(self._angle_pi())
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
        let [h, s, v, a] = self.into_hsva_components();
        [
            h.to_control(),
            s.to_control(),
            v.to_control(),
            a.to_control(),
        ]
    }
}

// A color in a config can be written a few ways, picked by shape:
//   [h, s, v, a]              four numbers, hsva
//   {rgb: [r, g, b], a}       rgba
//   {gray: g}                 a gray value
//   {hue: h}                  a fully-saturated hue
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlMurreletColor {
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Expr(ExprNode),
    Hsva([ControlF32; 4]),
    Hsv([ControlF32; 3]),
    // Named(String),
    Rgba {
        rgb: [ControlF32; 3],
        a: ControlF32,
    },
    Gray {
        gray: ControlF32,
    },
    Hue {
        hue: ControlF32,
    },
}

impl LivecodeFromWorld<MurreletColor> for ControlMurreletColor {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<MurreletColor> {
        match self {
            // by default, clamp saturation and value
            ControlMurreletColor::Hsva(hsva) => Ok(MurreletColor::hsva(
                hsva[0].o(w)?,
                clamp(hsva[1].o(w)?, 0.0, 1.0),
                clamp(hsva[2].o(w)?, 0.0, 1.0),
                hsva[3].o(w)?,
            )),
            ControlMurreletColor::Hsv(hsv) => Ok(MurreletColor::hsva(
                hsv[0].o(w)?,
                clamp(hsv[1].o(w)?, 0.0, 1.0),
                clamp(hsv[2].o(w)?, 0.0, 1.0),
                1.0,
            )),

            ControlMurreletColor::Rgba { rgb, a } => Ok(MurreletColor::rgba(
                rgb[0].o(w)?,
                rgb[1].o(w)?,
                rgb[2].o(w)?,
                a.o(w)?,
            )),
            ControlMurreletColor::Gray { gray } => Ok(MurreletColor::gray(gray.o(w)?)),
            ControlMurreletColor::Hue { hue } => Ok(MurreletColor::hue(hue.o(w)?)),
            // ControlMurreletColor::Named(name) => named_color_string(name),
            ControlMurreletColor::Expr(expr_node) => {
                // first check if it's a simple color name!
                if let Ok(c) = named_color_string(&expr_node.src) {
                    return Ok(c);
                }
                let v = expr_node.eval_with_context(w).to_lc_err()?;
                let v4 = expr_as_vec4(&v).to_lc_err()?;
                let c = MurreletColor::hsva(v4[0], v4[1], v4[2], v4[3]);
                Ok(c)
            }
        }
    }
}

pub fn named_color_string(name: &str) -> LivecodeResult<MurreletColor> {
    match name {
        "black" => Ok(MurreletColor::black()),
        "white" => Ok(MurreletColor::white()),
        "transparent" => Ok(MurreletColor::transparent()),
        _ => Err(format!("Missing color {}", name)).to_lc_err(),
    }
}

impl LivecodeToControl<ControlMurreletColor> for MurreletColor {
    fn to_control(&self) -> ControlMurreletColor {
        let [h, s, v, a] = self.into_hsva_components();
        ControlMurreletColor::Hsva([
            h.to_control(),
            s.to_control(),
            v.to_control(),
            a.to_control(),
        ])
    }
}

impl GetLivecodeIdentifiers for ControlMurreletColor {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        let nodes: Vec<&ControlF32> = match self {
            ControlMurreletColor::Hsva(hsva) => hsva.iter().collect(),
            ControlMurreletColor::Hsv(hsv) => hsv.iter().collect(),
            ControlMurreletColor::Rgba { rgb, a } => rgb.iter().chain(std::iter::once(a)).collect(),
            ControlMurreletColor::Gray { gray } => vec![gray],
            ControlMurreletColor::Hue { hue } => vec![hue],
            // ControlMurreletColor::Named(_) => vec![],
            ControlMurreletColor::Expr(expr_node) => {
                return expr_node
                    .iter_variable_identifiers()
                    .map(|x| LivecodeVariable::from_str(x))
                    .collect_vec();
            }
        };
        nodes
            .iter()
            .flat_map(|x| x.variable_identifiers())
            .collect::<HashSet<LivecodeVariable>>()
            .into_iter()
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        let nodes: Vec<&ControlF32> = match self {
            ControlMurreletColor::Hsva(hsva) => hsva.iter().collect(),
            ControlMurreletColor::Hsv(hsv) => hsv.iter().collect(),
            ControlMurreletColor::Rgba { rgb, a } => rgb.iter().chain(std::iter::once(a)).collect(),
            ControlMurreletColor::Gray { gray } => vec![gray],
            ControlMurreletColor::Hue { hue } => vec![hue],
            // ControlMurreletColor::Named(_) => vec![],
            ControlMurreletColor::Expr(expr_node) => {
                return expr_node
                    .iter_function_identifiers()
                    .map(|x| LivecodeFunction::from_str(x))
                    .collect_vec();
            }
        };
        nodes
            .iter()
            .flat_map(|x| x.function_identifiers())
            .collect::<HashSet<LivecodeFunction>>()
            .into_iter()
            .collect_vec()
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

impl LivecodeToControl<ControlF32> for f64 {
    fn to_control(&self) -> ControlF32 {
        ControlF32::Raw(*self as f32)
    }
}

impl<Source, Target> LivecodeToControl<Vec<Target>> for Vec<Source>
where
    Source: LivecodeToControl<Target>,
{
    fn to_control(&self) -> Vec<Target> {
        self.iter().map(|x| x.to_control()).collect_vec()
    }
}

impl LivecodeToControl<ControlLazyNodeF32> for LazyNodeF32 {
    fn to_control(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::new(self.n().cloned().unwrap())
    }
}

impl<T, ControlType> LivecodeToControl<Option<ControlType>> for Option<T>
where
    T: LivecodeToControl<ControlType> + Clone,
    ControlType: LivecodeFromWorld<T>,
{
    fn to_control(&self) -> Option<ControlType> {
        self.as_ref().map(|s| s.to_control())
    }
}

pub trait LivecodeToControlLazy<ControlT> {
    fn to_control_lazy(&self) -> ControlT;
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for f32 {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Float(*self)
    }
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for i32 {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Int(*self)
    }
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for u32 {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Float(*self as f32)
    }
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for u8 {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Float(*self as f32)
    }
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for usize {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Float(*self as f32)
    }
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for u64 {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Float(*self as f32)
    }
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for f64 {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Float(*self as f32)
    }
}

// we represent bools with lazynodef32
impl LivecodeToControlLazy<ControlLazyNodeF32> for bool {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Bool(*self)
    }
}

impl LivecodeToControlLazy<ControlLazyNodeF32> for AnglePi {
    fn to_control_lazy(&self) -> ControlLazyNodeF32 {
        ControlLazyNodeF32::Float(self._angle_pi())
    }
}

impl<Source, Target> LivecodeToControlLazy<Vec<Target>> for Vec<Source>
where
    Source: LivecodeToControlLazy<Target>,
{
    fn to_control_lazy(&self) -> Vec<Target> {
        self.iter().map(|x| x.to_control_lazy()).collect_vec()
    }
}

impl<T, ControlType> LivecodeToControlLazy<Option<ControlType>> for Option<T>
where
    T: LivecodeToControlLazy<ControlType> + Clone,
{
    fn to_control_lazy(&self) -> Option<ControlType> {
        self.as_ref().map(|s| s.to_control_lazy())
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
                .map(LivecodeVariable::from_str)
                .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        match self {
            ControlF32::Int(_) => vec![],
            ControlF32::Bool(_) => vec![],
            ControlF32::Float(_) => vec![],
            ControlF32::Expr(node) => node
                .iter_function_identifiers()
                .dedup()
                .map(LivecodeFunction::from_str)
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

impl<T> GetLivecodeIdentifiers for Vec<T>
where
    T: GetLivecodeIdentifiers,
{
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.iter()
            .flat_map(|x| x.variable_identifiers())
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.iter()
            .flat_map(|x| x.function_identifiers())
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

//
impl GetLivecodeIdentifiers for AdditionalContextNode {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.vars()
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

impl<T: GetLivecodeIdentifiers> GetLivecodeIdentifiers for Option<T> {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.as_ref()
            .map(|x| x.variable_identifiers())
            .unwrap_or(vec![])
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.as_ref()
            .map(|x| x.function_identifiers())
            .unwrap_or(vec![])
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
                .map(LivecodeVariable::from_str)
                .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        match self {
            ControlBool::Int(_) => vec![],
            ControlBool::Raw(_) => vec![],
            ControlBool::Float(_) => vec![],
            ControlBool::Expr(node) => node
                .iter_function_identifiers()
                .dedup()
                .map(LivecodeFunction::from_str)
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

pub fn _auto_default_vec3_0() -> [ControlF32; 3] {
    [
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
    ]
}
pub fn _auto_default_vec3_1() -> [ControlF32; 3] {
    [
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
    ]
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

pub fn _auto_default_color_0() -> ControlMurreletColor {
    ControlMurreletColor::Hsva([
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
        ControlF32::Raw(0.0),
    ])
}
pub fn _auto_default_color_1() -> ControlMurreletColor {
    ControlMurreletColor::Hsva([
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
        ControlF32::Raw(1.0),
    ])
}

pub fn _auto_default_bool_false() -> ControlBool {
    ControlBool::Raw(false)
}
pub fn _auto_default_bool_true() -> ControlBool {
    ControlBool::Raw(true)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Expr(ExprNode),
}

impl From<f32> for ControlF32 {
    fn from(v: f32) -> Self {
        ControlF32::Float(v)
    }
}
impl From<i32> for ControlF32 {
    fn from(v: i32) -> Self {
        ControlF32::Int(v)
    }
}
impl From<bool> for ControlF32 {
    fn from(v: bool) -> Self {
        ControlF32::Bool(v)
    }
}

impl ControlF32 {
    pub const ZERO: ControlF32 = ControlF32::Float(0.0);

    // for backwards compatibility
    #[allow(non_snake_case)]
    pub fn Raw(v: f32) -> ControlF32 {
        Self::Float(v)
    }

    pub fn force_from_str(s: &str) -> ControlF32 {
        match ExprNode::from_src(s.to_string()) {
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
            ControlF32::Expr(e) => e
                .eval_float_with_context(w)
                .map(|x| x as f32)
                .or_else(|_| e.eval_int_with_context(w).map(|b| b as f32))
                .or_else(|_| {
                    e.eval_boolean_with_context(w)
                        .map(|b| if b { 1.0 } else { -1.0 })
                        .map_err(|err| {
                            LivecodeError::EvalExpr("evalexpr err".to_string(), err.to_string())
                        })
                }),
        }
    }
}

impl Default for ControlBool {
    fn default() -> Self {
        Self::Raw(true)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlBool {
    Raw(bool),
    Int(i32),
    Float(f32),
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Expr(ExprNode),
}

impl From<bool> for ControlBool {
    fn from(v: bool) -> Self {
        ControlBool::Raw(v)
    }
}
impl From<i32> for ControlBool {
    fn from(v: i32) -> Self {
        ControlBool::Int(v)
    }
}
impl From<f32> for ControlBool {
    fn from(v: f32) -> Self {
        ControlBool::Float(v)
    }
}

impl ControlBool {
    pub fn force_from_str(s: &str) -> ControlBool {
        match ExprNode::from_src(s.to_string()) {
            Ok(e) => Self::Expr(e),
            Err(err) => {
                println!("{:?}", err);
                ControlBool::Raw(false)
            }
        }
    }

    pub fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<bool> {
        match self {
            ControlBool::Raw(b) => Ok(*b),
            ControlBool::Int(i) => Ok(*i > 0),
            ControlBool::Float(x) => Ok(*x > 0.0),

            ControlBool::Expr(e) => e
                .eval_boolean_with_context(w)
                .or_else(|_| e.eval_float_with_context(w).map(|b| b > 0.0))
                .or_else(|_| {
                    e.eval_int_with_context(w).map(|b| b > 0).map_err(|err| {
                        LivecodeError::EvalExpr("evalexpr err".to_string(), err.to_string())
                    })
                }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lazy::ControlLazyNodeF32;

    const EXPR: &str = "(1.0 + 2.0) * 3.0";

    #[test]
    fn control_f32_expr_round_trips() {
        let parsed = ControlF32::force_from_str(EXPR);
        let json = serde_json::to_string(&parsed).unwrap();
        let back: ControlF32 = serde_json::from_str(&json).unwrap();

        match back {
            ControlF32::Expr(ref e) => {
                assert_eq!(e.src, EXPR);
                let v: f32 = parsed.o_dummy().unwrap();
                assert_eq!(v, 9.0);
                let v_back: f32 = back.o_dummy().unwrap();
                assert_eq!(v_back, 9.0);
            }
            other => panic!("expected Expr, got {:?}", other),
        }
    }

    #[test]
    fn control_murrelet_color_reads_hsva_array() {
        let c: ControlMurreletColor = serde_yaml::from_str("[0.5, 1.0, 1.0, 0.8]").unwrap();
        assert!(matches!(c, ControlMurreletColor::Hsva(_)));
        let color = c.o_dummy().unwrap();
        let [h, s, v, a] = color.into_hsva_components();
        assert_eq!([h, s, v, a], [0.5, 1.0, 1.0, 0.8]);
    }

    #[test]
    fn control_murrelet_color_reads_rgba_map() {
        let c: ControlMurreletColor =
            serde_yaml::from_str("{rgb: [1.0, 0.0, 0.0], a: 0.5}").unwrap();
        assert!(matches!(c, ControlMurreletColor::Rgba { .. }));
        let color = c.o_dummy().unwrap();
        let [r, g, b, a] = color.into_rgba_components();
        assert!((r - 1.0).abs() < 1e-5, "r was {}", r);
        assert!(g.abs() < 1e-5, "g was {}", g);
        assert!(b.abs() < 1e-5, "b was {}", b);
        assert!((a - 0.5).abs() < 1e-5, "a was {}", a);
    }

    #[test]
    fn control_murrelet_color_reads_gray_map() {
        let c: ControlMurreletColor = serde_yaml::from_str("{gray: 0.5}").unwrap();
        assert!(matches!(c, ControlMurreletColor::Gray { .. }));
        let expected = MurreletColor::gray(0.5).into_hsva_components();
        assert_eq!(c.o_dummy().unwrap().into_hsva_components(), expected);
    }

    #[test]
    fn control_murrelet_color_reads_hue_map() {
        let c: ControlMurreletColor = serde_yaml::from_str("{hue: 0.3}").unwrap();
        assert!(matches!(c, ControlMurreletColor::Hue { .. }));
        let expected = MurreletColor::hue(0.3).into_hsva_components();
        assert_eq!(c.o_dummy().unwrap().into_hsva_components(), expected);
    }

    #[test]
    fn control_lazy_node_f32_expr_round_trips() {
        let parsed = ControlLazyNodeF32::Expr(ExprNode::from_src(EXPR.to_string()).unwrap());
        let json = serde_json::to_string(&parsed).unwrap();
        let back: ControlLazyNodeF32 = serde_json::from_str(&json).unwrap();

        match back {
            ControlLazyNodeF32::Expr(ref e) => {
                assert_eq!(e.src, EXPR);
                let lazy = back.o_dummy().unwrap();
                let v = lazy
                    .eval_with_ctx(&crate::expr::MixedEvalDefs::new())
                    .unwrap();
                assert_eq!(v, 9.0);
            }
            other => panic!("expected Expr, got {:?}", other),
        }
    }
}

impl LivecodeFromWorld<MurreletString> for ControlMurreletString {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<MurreletString> {
        self._o(w)
    }
}

impl LivecodeToControl<ControlMurreletString> for MurreletString {
    fn to_control(&self) -> ControlMurreletString {
        ControlMurreletString::Raw(self.as_string())
    }
}

impl LivecodeToControlLazy<crate::lazy::ControlLazyMurreletString> for MurreletString {
    fn to_control_lazy(&self) -> crate::lazy::ControlLazyMurreletString {
        crate::lazy::ControlLazyMurreletString::Raw(self.as_string())
    }
}

impl GetLivecodeIdentifiers for ControlMurreletString {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        match self {
            ControlMurreletString::Raw(_) => vec![],
            ControlMurreletString::Fmt { fill, .. }
            | ControlMurreletString::FmtFloat { fill, .. } => fill
                .iter()
                .flat_map(|f| f.variable_identifiers())
                .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        match self {
            ControlMurreletString::Raw(_) => vec![],
            ControlMurreletString::Fmt { fill, .. }
            | ControlMurreletString::FmtFloat { fill, .. } => fill
                .iter()
                .flat_map(|f| f.function_identifiers())
                .collect_vec(),
        }
    }
}

// a bit of help from claude, this will help us deserialize format strings!
#[derive(Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
enum ControlMurreletStringRepr {
    Raw(String),
    Fmt {
        fmt: String,
        #[serde(default)]
        fill: Vec<ControlF32>,
    },
    FmtFloat {
        fmt_f: String,
        #[serde(default)]
        fill: Vec<ControlF32>,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schemars", schemars(with = "ControlMurreletStringRepr"))]
#[serde(untagged)]
pub enum ControlMurreletString {
    Raw(String),
    Fmt {
        fmt: String,
        #[serde(default)]
        fill: Vec<ControlF32>,
    },
    FmtFloat {
        fmt_f: String,
        #[serde(default)]
        fill: Vec<ControlF32>,
    },
}

impl Default for ControlMurreletString {
    fn default() -> Self {
        ControlMurreletString::Raw(String::new())
    }
}

impl ControlMurreletString {
    pub fn raw(s: impl Into<String>) -> Self {
        ControlMurreletString::Raw(s.into())
    }

    pub fn _o(&self, w: &LivecodeWorldState) -> LivecodeResult<MurreletString> {
        match self {
            ControlMurreletString::Raw(s) => Ok(MurreletString::new(s.clone())),
            ControlMurreletString::Fmt { fmt, fill } => {
                let filled = fill
                    .iter()
                    .map(|f| f.o(w).map(|x: f32| x.round() as i64))
                    .collect::<Result<Vec<_>, _>>()?;
                let out = fill_fmt(fmt, &filled)?;
                Ok(MurreletString::new(out))
            }
            ControlMurreletString::FmtFloat { fmt_f, fill } => {
                let filled = fill
                    .iter()
                    .map(|f| f.o(w).map(|x: f32| x as f64))
                    .collect::<Result<Vec<_>, _>>()?;
                let out = fill_fmt_f64(fmt_f, &filled)?;
                Ok(MurreletString::new(out))
            }
        }
    }
}

impl<'de> Deserialize<'de> for ControlMurreletString {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ControlMurreletStringRepr::deserialize(d)? {
            ControlMurreletStringRepr::Raw(s) => {
                match desugar_interp(&s).map_err(serde::de::Error::custom)? {
                    None => Ok(ControlMurreletString::Raw(s)),
                    Some((lit, exprs)) if exprs.is_empty() => Ok(ControlMurreletString::Raw(lit)),
                    Some((fmt, exprs)) => {
                        let fill = exprs
                            .into_iter()
                            .map(|src| ExprNode::from_src(src).map(ControlF32::Expr))
                            .collect::<LivecodeResult<Vec<_>>>()
                            .map_err(serde::de::Error::custom)?;
                        Ok(ControlMurreletString::Fmt { fmt, fill })
                    }
                }
            }
            ControlMurreletStringRepr::Fmt { fmt, fill } => {
                Ok(ControlMurreletString::Fmt { fmt, fill })
            }
            ControlMurreletStringRepr::FmtFloat { fmt_f, fill } => {
                Ok(ControlMurreletString::FmtFloat { fmt_f, fill })
            }
        }
    }
}

impl Serialize for ControlMurreletString {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ControlMurreletString::Raw(v) => {
                // escape braces!
                ControlMurreletStringRepr::Raw(escape_braces(v)).serialize(s)
            }
            ControlMurreletString::Fmt { fmt, fill } => ControlMurreletStringRepr::Fmt {
                fmt: fmt.clone(),
                fill: fill.clone(),
            }
            .serialize(s),
            ControlMurreletString::FmtFloat { fmt_f, fill } => {
                ControlMurreletStringRepr::FmtFloat {
                    fmt_f: fmt_f.clone(),
                    fill: fill.clone(),
                }
                .serialize(s)
            }
        }
    }
}

pub(crate) fn escape_braces(s: &str) -> String {
    if !s.contains('{') && !s.contains('}') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '{' => out.push_str("{{"),
            '}' => out.push_str("}}"),
            other => out.push(other),
        }
    }
    out
}

// chatbots can write my parsers for me.
pub(crate) fn desugar_interp(s: &str) -> LivecodeResult<Option<(String, Vec<String>)>> {
    if !s.contains('{') && !s.contains('}') {
        return Ok(None);
    }

    let mut out = String::with_capacity(s.len());
    let mut exprs = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    out.push_str("{{");
                } else {
                    let mut inner = String::new();
                    let mut closed = false;
                    for c2 in chars.by_ref() {
                        if c2 == '}' {
                            closed = true;
                            break;
                        }
                        inner.push(c2);
                    }
                    if !closed {
                        return Err(LivecodeError::Raw(format!(
                            "MurreletString {:?} has an unmatched {{",
                            s
                        )));
                    }
                    let inner = inner.trim();
                    if inner.is_empty() {
                        return Err(LivecodeError::Raw(format!(
                            "empty {{}} in {:?} — inline interpolation needs an expression \
                             (`{{c_i}}`); the positional form is `{{ fmt: \"…{{}}\", fill: [c_i] }}`",
                            s
                        )));
                    }
                    if inner.contains(':') {
                        return Err(LivecodeError::Raw(format!(
                            "format specs like `{{x:f}}` aren't supported yet (in {:?})",
                            s
                        )));
                    }
                    out.push_str("{}");
                    exprs.push(inner.to_string());
                }
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                    out.push_str("}}");
                } else {
                    return Err(LivecodeError::Raw(format!(
                        "MurreletString {:?} has an unmatched }}",
                        s
                    )));
                }
            }
            other => out.push(other),
        }
    }

    if exprs.is_empty() {
        // only escapes — unescape back to a plain literal for Raw
        Ok(Some((out.replace("{{", "{").replace("}}", "}"), exprs)))
    } else {
        Ok(Some((out, exprs)))
    }
}

fn escape_lit(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// contents of a backtick token, or of a get("…") first arg → `"fmt", e1, e2`
fn interp_to_get_args(contents: &str) -> LivecodeResult<String> {
    match desugar_interp(contents)? {
        None => Ok(format!("\"{}\"", escape_lit(contents))),
        Some((lit, exprs)) if exprs.is_empty() => Ok(format!("\"{}\"", escape_lit(&lit))),
        Some((fmt, exprs)) => Ok(format!("\"{}\", {}", escape_lit(&fmt), exprs.join(", "))),
    }
}

fn is_get_first_arg(out: &str) -> bool {
    let t = out.trim_end();
    ["get(", "get_or("].iter().any(|name| {
        t.strip_suffix(name).is_some_and(|head| {
            head.chars()
                .last()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_')
        })
    })
}

pub(crate) fn desugar_expr_src(src: &str) -> LivecodeResult<Option<String>> {
    if !src.contains('`') && !src.contains("get") {
        return Ok(None); // fast path, no allocation
    }

    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut changed = false;
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '`' => {
                // allow an optional `f` prefix (f`…`) so a config can start a
                // plain YAML scalar with it — ` alone is a YAML reserved indicator
                if out.ends_with('f') {
                    let head = &out[..out.len() - 1];
                    if head
                        .chars()
                        .last()
                        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
                    {
                        out.pop();
                    }
                }

                let start = i + 1;
                let end = (start..chars.len())
                    .find(|&j| chars[j] == '`')
                    .ok_or_else(|| {
                        LivecodeError::Raw(format!("unterminated ` in expression {:?}", src))
                    })?;
                let contents: String = chars[start..end].iter().collect();
                out.push_str(&format!("get({})", interp_to_get_args(&contents)?));
                changed = true;
                i = end + 1;
            }
            '"' => {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != '"' {
                    end += if chars[end] == '\\' { 2 } else { 1 };
                }
                if end >= chars.len() {
                    return Err(LivecodeError::Raw(format!(
                        "unterminated \" in expression {:?}",
                        src
                    )));
                }
                let lit: String = chars[start..end].iter().collect();

                let has_bare = lit.contains("{}");
                let has_expr = lit.replace("{}", "").contains('{');
                if has_bare && has_expr {
                    return Err(LivecodeError::Raw(format!(
                        "can't mix `{{expr}}` and positional `{{}}` in {:?} (ambiguous fill order)",
                        src
                    )));
                }

                if is_get_first_arg(&out) && has_expr {
                    out.push_str(&interp_to_get_args(&lit)?);
                    changed = true;
                } else {
                    out.push('"');
                    out.push_str(&lit);
                    out.push('"');
                }
                i = end + 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }

    Ok(if changed { Some(out) } else { None })
}

// substitute `{}` placeholders in `fmt` positionally (Rust-style, left-to-right)
// with the integer fill values. `{{` and `}}` are literal braces.
pub(crate) fn fill_fmt(fmt: &str, fill: &[i64]) -> LivecodeResult<String> {
    let mut out = String::new();
    let mut next = 0;
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    out.push('{');
                } else if chars.peek() == Some(&'}') {
                    chars.next();
                    let v = fill.get(next).ok_or_else(|| {
                        LivecodeError::Raw(format!(
                            "MurreletString fmt {:?} has more placeholders than fills ({})",
                            fmt,
                            fill.len()
                        ))
                    })?;
                    out.push_str(&v.to_string());
                    next += 1;
                } else {
                    return Err(LivecodeError::Raw(format!(
                        "MurreletString fmt {:?} only supports empty {{}} placeholders",
                        fmt
                    )));
                }
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                    out.push('}');
                } else {
                    return Err(LivecodeError::Raw(format!(
                        "MurreletString fmt {:?} has an unmatched }}",
                        fmt
                    )));
                }
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

pub(crate) fn fill_fmt_f64(fmt: &str, fill: &[f64]) -> LivecodeResult<String> {
    let mut out = String::new();
    let mut next = 0;
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    out.push('{');
                } else if chars.peek() == Some(&'}') {
                    chars.next();
                    let v = fill.get(next).ok_or_else(|| {
                        LivecodeError::Raw(format!(
                            "MurreletString fmt_f {:?} has more placeholders than fills ({})",
                            fmt,
                            fill.len()
                        ))
                    })?;
                    out.push_str(&v.to_string());
                    next += 1;
                } else {
                    return Err(LivecodeError::Raw(format!(
                        "MurreletString fmt_f {:?} only supports empty {{}} placeholders",
                        fmt
                    )));
                }
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                    out.push('}');
                } else {
                    return Err(LivecodeError::Raw(format!(
                        "MurreletString fmt_f {:?} has an unmatched }}",
                        fmt
                    )));
                }
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

// hax
pub type Controlf32 = ControlF32;
pub type ControlLazyf32 = ControlLazyNodeF32;

pub type Controlusize = ControlF32;
pub type ControlLazyusize = ControlLazyNodeF32;

pub type Controlbool = ControlBool;
pub type ControlLazybool = ControlLazyNodeF32;

#[macro_export]
macro_rules! livecode_default {
    ($name:ident = $ctrl_ty:ty: $value:expr, $ctrl_lazy_ty:ty: $lazy_value:expr) => {
        murrelet_livecode::paste::paste! {
            pub fn $name() -> $ctrl_ty {
                $value
            }

            pub fn [<$name _lazy>]() -> $ctrl_lazy_ty {
                $lazy_value
            }
        }
    };

    ($name:ident: $ty:ty = $value:expr) => {
        murrelet_livecode::paste::paste! {
            fn [<$name _val>]() -> $ty {
                $value
            }

            pub fn $name() -> [<Control $ty>] {
                [<$name _val>]().to_control()
            }

            pub fn [<$name _lazy>]() -> [<ControlLazy $ty>] {
                [<$name _val>]().to_control_lazy()
            }
        }
    };
}
