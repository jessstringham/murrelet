use std::sync::Arc;

use crate::{
    expr::{ExprWorldContextValues, MixedEvalDefs, ToMixedDefs},
    livecode::{
        ExprNode, GetLivecodeIdentifiers, LivecodeFromWorld, LivecodeFunction, LivecodeToControl,
        LivecodeToControlLazy, LivecodeVariable,
    },
    nestedit::{NestEditable, NestedMod},
    state::{LivecodeWorldState, WorldWithLocalVariables},
    types::{LivecodeError, LivecodeResult},
};
use evalexpr::Node;

use glam::Vec2;
use itertools::Itertools;
use lerpable::IsLerpingMethod;
use lerpable::{Lerpable, step};
use murrelet_common::{IdxInRange, LivecodeValue, MurreletColor, MurreletString};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlLazyNodeF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Expr(ExprNode),
}

impl From<f32> for ControlLazyNodeF32 {
    fn from(v: f32) -> Self {
        ControlLazyNodeF32::Float(v)
    }
}
impl From<i32> for ControlLazyNodeF32 {
    fn from(v: i32) -> Self {
        ControlLazyNodeF32::Int(v)
    }
}
impl From<bool> for ControlLazyNodeF32 {
    fn from(v: bool) -> Self {
        ControlLazyNodeF32::Bool(v)
    }
}

impl ControlLazyNodeF32 {
    pub const ZERO: Self = ControlLazyNodeF32::Float(0.0);

    pub fn new(n: Node) -> Self {
        Self::Expr(ExprNode::from_node(n))
    }

    pub fn new_f32(n: f32) -> Self {
        Self::Float(n)
    }

    fn result(&self) -> Result<f32, LivecodeError> {
        match self {
            ControlLazyNodeF32::Int(d) => Ok(*d as f32),
            ControlLazyNodeF32::Bool(d) => Ok(if *d { 1.0 } else { -1.0 }),
            ControlLazyNodeF32::Float(d) => Ok(*d),
            ControlLazyNodeF32::Expr(_) => Err(LivecodeError::Raw("result on a expr".to_owned())),
        }
    }
}

impl LivecodeFromWorld<LazyNodeF32> for ControlLazyNodeF32 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(LazyNodeF32::new(self.clone(), w))
    }
}

impl GetLivecodeIdentifiers for ControlLazyNodeF32 {
    fn variable_identifiers(&self) -> Vec<crate::livecode::LivecodeVariable> {
        match self {
            ControlLazyNodeF32::Expr(node) => node
                .iter_variable_identifiers()
                .sorted()
                .dedup()
                .map(LivecodeVariable::from_str)
                .collect_vec(),
            _ => vec![],
        }
    }

    fn function_identifiers(&self) -> Vec<crate::livecode::LivecodeFunction> {
        match self {
            ControlLazyNodeF32::Expr(node) => node
                .iter_function_identifiers()
                .sorted()
                .dedup()
                .map(LivecodeFunction::from_str)
                .collect_vec(),
            _ => vec![],
        }
    }
}

// todo, figure out how to only build this context once per unitcell/etc
#[derive(Debug, Clone)]
pub struct LazyNodeF32Inner {
    n: Arc<Node>,                   // what will be evaluated!
    world: WorldWithLocalVariables, // this is a reference :D
}
impl LazyNodeF32Inner {
    pub fn new(n: Node, world: LivecodeWorldState) -> Self {
        Self {
            n: Arc::new(n),
            world: world.to_local(),
        }
    }

    // options to add more details...
    pub fn add_more_defs<M: ToMixedDefs>(&self, more_defs: &M) -> Self {
        let c = self.clone();
        c.add_expr_values(more_defs.to_mixed_def().expr_vals())
    }

    pub fn add_expr_values(&self, more_vals: &ExprWorldContextValues) -> Self {
        let mut c = self.clone();
        c.world.update_with_simple_defs(more_vals);
        c
    }

    // internal function to build the ctx
    fn build_ctx(&self) -> &WorldWithLocalVariables {
        &self.world
    }

    // what you'll use
    pub fn eval(&self) -> LivecodeResult<f32> {
        let ctx = self.build_ctx();

        self.n
            .eval_float_with_context(ctx)
            .or_else(|_| self.n.eval_int_with_context(ctx).map(|x| x as f64))
            .or_else(|_| {
                self.n
                    .eval_boolean_with_context(ctx)
                    .map(|x| if x { 1.0 } else { -1.0 })
            })
            .map(|x| x as f32)
            .map_err(|err| LivecodeError::EvalExpr("error evaluating lazy".to_string(), err))
    }
}

// // expr that we can add things
#[derive(Debug, Clone, Default)]
pub enum LazyNodeF32 {
    #[default]
    Uninitialized,
    Node(LazyNodeF32Inner),
    NoCtxNode(ControlLazyNodeF32), // this hasn't been evaluated with .o()? yet
}

impl LazyNodeF32 {
    pub fn new(def: ControlLazyNodeF32, world: &LivecodeWorldState) -> Self {
        match def {
            ControlLazyNodeF32::Expr(n) => {
                Self::Node(LazyNodeF32Inner::new(n.node().clone(), world.clone()))
            }
            _ => Self::NoCtxNode(def),
        }
    }

    pub fn simple_number(val: f32) -> Self {
        Self::new(
            ControlLazyNodeF32::Float(val),
            &LivecodeWorldState::new_dummy(),
        )
    }

    pub fn n(&self) -> Option<&Node> {
        match self {
            LazyNodeF32::Uninitialized => None,
            LazyNodeF32::Node(n) => Some(&n.n),
            LazyNodeF32::NoCtxNode(_) => None,
        }
    }

    pub fn eval_with_ctx<M: ToMixedDefs>(&self, more_defs: &M) -> LivecodeResult<f32> {
        // update ctx
        let with_more_ctx = self.add_more_defs(more_defs)?;

        match with_more_ctx {
            LazyNodeF32::Uninitialized => {
                Err(LivecodeError::Raw("uninitialized lazy node".to_owned()))
            }
            LazyNodeF32::Node(v) => v.eval(),
            LazyNodeF32::NoCtxNode(v) => v.result(),
        }
    }

    pub fn add_more_defs<M: ToMixedDefs>(&self, more_defs: &M) -> LivecodeResult<Self> {
        match self {
            LazyNodeF32::Uninitialized => {
                Err(LivecodeError::Raw("uninitialized lazy node".to_owned()))
            }
            LazyNodeF32::Node(v) => Ok(LazyNodeF32::Node(v.add_more_defs(more_defs))),
            LazyNodeF32::NoCtxNode(_) => Ok(self.clone()), // hmm, this is dropping more_defs...
        }
    }

    /// short-hand to evaluate an index with the provided prefix
    pub fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<f32> {
        match self {
            LazyNodeF32::Uninitialized => {
                Err(LivecodeError::Raw("uninitialized lazy node".to_owned()))
            }
            LazyNodeF32::Node(v) => {
                let vals =
                    ExprWorldContextValues::new_from_idx(idx).with_prefix(&format!("{}_", prefix));
                v.add_expr_values(&vals).eval()
            }
            LazyNodeF32::NoCtxNode(v) => v.result(),
        }
    }

    pub fn node(&self) -> LivecodeResult<&LazyNodeF32Inner> {
        if let Self::Node(v) = self {
            Ok(v)
        } else {
            Err(LivecodeError::Raw(
                "trying to use uninitialized lazy node".to_owned(),
            ))
        }
    }

    /// prints out the variables
    pub fn variable_names(&self) -> LivecodeResult<Vec<String>> {
        match self {
            LazyNodeF32::Uninitialized => Err(LivecodeError::Raw("not initialized".to_owned())),
            LazyNodeF32::Node(c) => Ok(c.build_ctx().variable_names()),
            LazyNodeF32::NoCtxNode(_) => Err(LivecodeError::Raw("no ctx".to_owned())),
        }
    }

    pub fn eval_with_xy(&self, xy: glam::Vec2) -> LivecodeResult<f32> {
        let expr = ExprWorldContextValues::new(vec![
            ("x".to_string(), LivecodeValue::float(xy.x)),
            ("y".to_string(), LivecodeValue::float(xy.y)),
        ]);

        self.eval_with_ctx(&expr)
    }

    fn new_func(x: &str) -> LazyNodeF32 {
        LazyNodeF32::new(
            ControlLazyNodeF32::Expr(ExprNode::from_src(x.to_string()).unwrap()),
            &LivecodeWorldState::new_dummy(),
        )
    }
}

impl Lerpable for LazyNodeF32 {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        step(self, other, pct)
    }
}

pub trait IsLazy
where
    Self: Sized + Clone,
{
    type Target;

    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<Self::Target>;

    fn with_more_defs(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<Self>;

    // without a _, like unitcell..
    fn eval_idx_(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<Self::Target> {
        let vals = ExprWorldContextValues::new_from_idx(idx).with_prefix(prefix);

        self.eval_lazy(&MixedEvalDefs::new_from_expr(vals))
    }

    // backwards compatible
    fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<Self::Target> {
        self.eval_idx_(idx, &format!("{}_", prefix))
    }
}

impl IsLazy for LazyNodeF32 {
    type Target = f32;
    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<f32> {
        self.eval_with_ctx(expr)
    }

    fn with_more_defs(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<Self> {
        self.add_more_defs(more_defs)
    }
}

impl<Source, VecElemTarget> IsLazy for Vec<Source>
where
    Source: IsLazy<Target = VecElemTarget>,
{
    type Target = Vec<VecElemTarget>;
    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<Vec<VecElemTarget>> {
        self.iter().map(|x| x.eval_lazy(expr)).collect()
    }

    fn with_more_defs(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<Self> {
        self.iter()
            .map(|item| item.with_more_defs(more_defs))
            .collect::<LivecodeResult<Vec<_>>>()
    }
}

impl<T> crate::unitcells::UnitCellCreator for T
where
    T: IsLazy,
    T::Target: crate::unitcells::UnitCellCreator,
{
    fn to_unit_cell_ctxs(&self) -> Vec<crate::unitcells::UnitCellContext> {
        unimplemented!("not sure how to do lazy unitcells yet...")
    }
}

#[derive(Clone, Debug, Default)]
pub struct LazyVec2 {
    x: LazyNodeF32,
    y: LazyNodeF32,
}

impl LazyVec2 {
    pub fn new(x: LazyNodeF32, y: LazyNodeF32) -> Self {
        Self { x, y }
    }

    pub fn from_vec2(v: Vec2) -> Self {
        Self::new(
            LazyNodeF32::simple_number(v.x),
            LazyNodeF32::simple_number(v.y),
        )
    }

    pub fn new_funcs(x: &str, y: &str) -> Self {
        Self::new(LazyNodeF32::new_func(x), LazyNodeF32::new_func(y))
    }
}

impl NestEditable for LazyVec2 {
    fn nest_update(&self, _mods: NestedMod) -> Self {
        self.clone() // noop
    }

    fn nest_get(&self, _getter: &[&str]) -> LivecodeResult<String> {
        Err(LivecodeError::NestGetExtra("LazyNodeF32".to_owned())) // maybe in the future!
    }
}

impl NestEditable for LazyVec3 {
    fn nest_update(&self, _mods: NestedMod) -> Self {
        self.clone()
    }

    fn nest_get(&self, _getter: &[&str]) -> LivecodeResult<String> {
        Err(LivecodeError::NestGetExtra("LazyVec3".to_owned()))
    }
}

impl NestEditable for LazyMurreletColor {
    fn nest_update(&self, _mods: NestedMod) -> Self {
        self.clone()
    }

    fn nest_get(&self, _getter: &[&str]) -> LivecodeResult<String> {
        Err(LivecodeError::NestGetExtra("LazyMurreletColor".to_owned()))
    }
}

impl NestEditable for LazyMurreletString {
    fn nest_update(&self, _mods: NestedMod) -> Self {
        self.clone()
    }

    fn nest_get(&self, _getter: &[&str]) -> LivecodeResult<String> {
        Err(LivecodeError::NestGetExtra("LazyMurreletString".to_owned()))
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ControlLazyVec2(Vec<ControlLazyNodeF32>);

impl ControlLazyVec2 {
    pub fn new(x: ControlLazyNodeF32, y: ControlLazyNodeF32) -> Self {
        Self(vec![x, y])
    }
}
impl LivecodeFromWorld<LazyVec2> for ControlLazyVec2 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyVec2> {
        Ok(LazyVec2::new(self.0[0].o(w)?, self.0[1].o(w)?))
    }
}

impl GetLivecodeIdentifiers for ControlLazyVec2 {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.0
            .iter()
            .flat_map(|f| f.variable_identifiers())
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.0
            .iter()
            .flat_map(|f| f.function_identifiers())
            .collect_vec()
    }
}

impl LivecodeToControl<ControlLazyVec2> for LazyVec2 {
    fn to_control(&self) -> ControlLazyVec2 {
        ControlLazyVec2(vec![self.x.to_control(), self.y.to_control()])
    }
}

impl LivecodeToControlLazy<ControlLazyVec2> for Vec2 {
    fn to_control_lazy(&self) -> ControlLazyVec2 {
        ControlLazyVec2::new(self.x.to_control_lazy(), self.y.to_control_lazy())
    }
}

impl IsLazy for LazyVec2 {
    type Target = glam::Vec2;

    fn eval_lazy(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        Ok(glam::vec2(self.x.eval_lazy(ctx)?, self.y.eval_lazy(ctx)?))
    }

    fn with_more_defs(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self> {
        Ok(LazyVec2::new(
            self.x.with_more_defs(ctx)?,
            self.y.with_more_defs(ctx)?,
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub struct LazyVec3 {
    x: LazyNodeF32,
    y: LazyNodeF32,
    z: LazyNodeF32,
}

impl LazyVec3 {
    pub fn new(x: LazyNodeF32, y: LazyNodeF32, z: LazyNodeF32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ControlLazyVec3(Vec<ControlLazyNodeF32>);
impl LivecodeFromWorld<LazyVec3> for ControlLazyVec3 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyVec3> {
        Ok(LazyVec3::new(
            self.0[0].o(w)?,
            self.0[1].o(w)?,
            self.0[2].o(w)?,
        ))
    }
}

impl GetLivecodeIdentifiers for ControlLazyVec3 {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.0
            .iter()
            .flat_map(|f| f.variable_identifiers())
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.0
            .iter()
            .flat_map(|f| f.function_identifiers())
            .collect_vec()
    }
}

impl LivecodeToControl<ControlLazyVec3> for LazyVec3 {
    fn to_control(&self) -> ControlLazyVec3 {
        ControlLazyVec3(vec![
            self.x.to_control(),
            self.y.to_control(),
            self.z.to_control(),
        ])
    }
}

impl LivecodeToControlLazy<ControlLazyVec3> for glam::Vec3 {
    fn to_control_lazy(&self) -> ControlLazyVec3 {
        ControlLazyVec3(vec![
            self.x.to_control_lazy(),
            self.y.to_control_lazy(),
            self.z.to_control_lazy(),
        ])
    }
}

impl IsLazy for LazyVec3 {
    type Target = glam::Vec3;

    fn eval_lazy(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        Ok(glam::vec3(
            self.x.eval_lazy(ctx)?,
            self.y.eval_lazy(ctx)?,
            self.z.eval_lazy(ctx)?,
        ))
    }

    fn with_more_defs(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self> {
        Ok(LazyVec3::new(
            self.x.with_more_defs(ctx)?,
            self.y.with_more_defs(ctx)?,
            self.z.with_more_defs(ctx)?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LazyColorMode {
    #[default]
    Hsva,
    Rgba,
}

#[derive(Clone, Debug, Default)]
pub struct LazyMurreletColor {
    mode: LazyColorMode,
    c0: LazyNodeF32,
    c1: LazyNodeF32,
    c2: LazyNodeF32,
    a: LazyNodeF32,
}

impl LazyMurreletColor {
    pub fn new(h: LazyNodeF32, s: LazyNodeF32, v: LazyNodeF32, a: LazyNodeF32) -> Self {
        Self {
            mode: LazyColorMode::Hsva,
            c0: h,
            c1: s,
            c2: v,
            a,
        }
    }

    pub fn new_rgba(r: LazyNodeF32, g: LazyNodeF32, b: LazyNodeF32, a: LazyNodeF32) -> Self {
        Self {
            mode: LazyColorMode::Rgba,
            c0: r,
            c1: g,
            c2: b,
            a,
        }
    }

    pub fn white() -> Self {
        Self::new(
            LazyNodeF32::simple_number(0.0),
            LazyNodeF32::simple_number(0.0),
            LazyNodeF32::simple_number(1.0),
            LazyNodeF32::simple_number(1.0),
        )
    }

    pub fn black() -> Self {
        Self::new(
            LazyNodeF32::simple_number(0.0),
            LazyNodeF32::simple_number(0.0),
            LazyNodeF32::simple_number(0.0),
            LazyNodeF32::simple_number(1.0),
        )
    }
}

// Same shape as the eager `ControlMurreletColor`: four numbers `[h, s, v, a]`,
// or an `{rgb: [r, g, b], a}` map, each entry a lazy node.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlLazyMurreletColor {
    Hsva(Vec<ControlLazyNodeF32>),
    Rgba {
        rgb: [ControlLazyNodeF32; 3],
        a: ControlLazyNodeF32,
    },
    Gray {
        gray: ControlLazyNodeF32,
    },
    Hue {
        hue: ControlLazyNodeF32,
    },
}

impl Default for ControlLazyMurreletColor {
    fn default() -> Self {
        ControlLazyMurreletColor::Hsva(Vec::new())
    }
}

impl ControlLazyMurreletColor {
    pub fn new_default(h: f32, s: f32, v: f32, a: f32) -> Self {
        ControlLazyMurreletColor::Hsva(vec![
            ControlLazyNodeF32::new_f32(h),
            ControlLazyNodeF32::new_f32(s),
            ControlLazyNodeF32::new_f32(v),
            ControlLazyNodeF32::new_f32(a),
        ])
    }

    fn nodes(&self) -> Vec<&ControlLazyNodeF32> {
        match self {
            ControlLazyMurreletColor::Hsva(hsva) => hsva.iter().collect(),
            ControlLazyMurreletColor::Rgba { rgb, a } => {
                rgb.iter().chain(std::iter::once(a)).collect()
            }
            ControlLazyMurreletColor::Gray { gray } => vec![gray],
            ControlLazyMurreletColor::Hue { hue } => vec![hue],
        }
    }
}

impl LivecodeFromWorld<LazyMurreletColor> for ControlLazyMurreletColor {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyMurreletColor> {
        match self {
            ControlLazyMurreletColor::Hsva(hsva) => Ok(LazyMurreletColor::new(
                hsva[0].o(w)?,
                hsva[1].o(w)?,
                hsva[2].o(w)?,
                hsva[3].o(w)?,
            )),
            ControlLazyMurreletColor::Rgba { rgb, a } => Ok(LazyMurreletColor::new_rgba(
                rgb[0].o(w)?,
                rgb[1].o(w)?,
                rgb[2].o(w)?,
                a.o(w)?,
            )),
            // gray g == rgba(g, g, g, 1); hue h == hsva(h, 1, 1, 1)
            ControlLazyMurreletColor::Gray { gray } => {
                let g = gray.o(w)?;
                Ok(LazyMurreletColor::new_rgba(
                    g.clone(),
                    g.clone(),
                    g,
                    LazyNodeF32::simple_number(1.0),
                ))
            }
            ControlLazyMurreletColor::Hue { hue } => Ok(LazyMurreletColor::new(
                hue.o(w)?,
                LazyNodeF32::simple_number(1.0),
                LazyNodeF32::simple_number(1.0),
                LazyNodeF32::simple_number(1.0),
            )),
        }
    }
}

impl GetLivecodeIdentifiers for ControlLazyMurreletColor {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.nodes()
            .iter()
            .flat_map(|f| f.variable_identifiers())
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.nodes()
            .iter()
            .flat_map(|f| f.function_identifiers())
            .collect_vec()
    }
}

impl LivecodeToControl<ControlLazyMurreletColor> for LazyMurreletColor {
    fn to_control(&self) -> ControlLazyMurreletColor {
        match self.mode {
            LazyColorMode::Hsva => ControlLazyMurreletColor::Hsva(vec![
                self.c0.to_control(),
                self.c1.to_control(),
                self.c2.to_control(),
                self.a.to_control(),
            ]),
            LazyColorMode::Rgba => ControlLazyMurreletColor::Rgba {
                rgb: [
                    self.c0.to_control(),
                    self.c1.to_control(),
                    self.c2.to_control(),
                ],
                a: self.a.to_control(),
            },
        }
    }
}

impl LivecodeToControlLazy<ControlLazyMurreletColor> for MurreletColor {
    fn to_control_lazy(&self) -> ControlLazyMurreletColor {
        let [h, s, v, a] = self.into_hsva_components();
        ControlLazyMurreletColor::Hsva(vec![
            h.to_control_lazy(),
            s.to_control_lazy(),
            v.to_control_lazy(),
            a.to_control_lazy(),
        ])
    }
}

impl IsLazy for LazyMurreletColor {
    type Target = MurreletColor;

    fn eval_lazy(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        let c0 = self.c0.eval_lazy(ctx)?;
        let c1 = self.c1.eval_lazy(ctx)?;
        let c2 = self.c2.eval_lazy(ctx)?;
        let a = self.a.eval_lazy(ctx)?;
        Ok(match self.mode {
            LazyColorMode::Hsva => MurreletColor::hsva(c0, c1, c2, a),
            LazyColorMode::Rgba => MurreletColor::rgba(c0, c1, c2, a),
        })
    }

    fn with_more_defs(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self> {
        Ok(LazyMurreletColor {
            mode: self.mode,
            c0: self.c0.with_more_defs(ctx)?,
            c1: self.c1.with_more_defs(ctx)?,
            c2: self.c2.with_more_defs(ctx)?,
            a: self.a.with_more_defs(ctx)?,
        })
    }
}

// lazy sibling of ControlMurreletString. holds the fmt + lazy fills so the
// substitution can run at eval_lazy time against the deferred defs. mirrors
// LazyMurreletColor.
#[derive(Clone, Debug)]
pub enum LazyMurreletString {
    Raw(String),
    Fmt {
        fmt: String,
        fill: Vec<LazyNodeF32>,
    },
    FmtFloat {
        fmt_f: String,
        fill: Vec<LazyNodeF32>,
    },
}

impl Default for LazyMurreletString {
    fn default() -> Self {
        LazyMurreletString::Raw(String::new())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlLazyMurreletString {
    Raw(String),
    Fmt {
        fmt: String,
        #[serde(default)]
        fill: Vec<ControlLazyNodeF32>,
    },
    FmtFloat {
        fmt_f: String,
        #[serde(default)]
        fill: Vec<ControlLazyNodeF32>,
    },
}

impl Default for ControlLazyMurreletString {
    fn default() -> Self {
        ControlLazyMurreletString::Raw(String::new())
    }
}

impl LivecodeFromWorld<LazyMurreletString> for ControlLazyMurreletString {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyMurreletString> {
        match self {
            ControlLazyMurreletString::Raw(s) => Ok(LazyMurreletString::Raw(s.clone())),
            ControlLazyMurreletString::Fmt { fmt, fill } => Ok(LazyMurreletString::Fmt {
                fmt: fmt.clone(),
                fill: fill.iter().map(|f| f.o(w)).collect::<Result<Vec<_>, _>>()?,
            }),
            ControlLazyMurreletString::FmtFloat { fmt_f, fill } => {
                Ok(LazyMurreletString::FmtFloat {
                    fmt_f: fmt_f.clone(),
                    fill: fill.iter().map(|f| f.o(w)).collect::<Result<Vec<_>, _>>()?,
                })
            }
        }
    }
}

impl GetLivecodeIdentifiers for ControlLazyMurreletString {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        match self {
            ControlLazyMurreletString::Raw(_) => vec![],
            ControlLazyMurreletString::Fmt { fill, .. }
            | ControlLazyMurreletString::FmtFloat { fill, .. } => fill
                .iter()
                .flat_map(|f| f.variable_identifiers())
                .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        match self {
            ControlLazyMurreletString::Raw(_) => vec![],
            ControlLazyMurreletString::Fmt { fill, .. }
            | ControlLazyMurreletString::FmtFloat { fill, .. } => fill
                .iter()
                .flat_map(|f| f.function_identifiers())
                .collect_vec(),
        }
    }
}

impl LivecodeToControl<ControlLazyMurreletString> for LazyMurreletString {
    fn to_control(&self) -> ControlLazyMurreletString {
        match self {
            LazyMurreletString::Raw(s) => ControlLazyMurreletString::Raw(s.clone()),
            LazyMurreletString::Fmt { fmt, fill } => ControlLazyMurreletString::Fmt {
                fmt: fmt.clone(),
                fill: fill.iter().map(|f| f.to_control()).collect_vec(),
            },
            LazyMurreletString::FmtFloat { fmt_f, fill } => {
                ControlLazyMurreletString::FmtFloat {
                    fmt_f: fmt_f.clone(),
                    fill: fill.iter().map(|f| f.to_control()).collect_vec(),
                }
            }
        }
    }
}

impl IsLazy for LazyMurreletString {
    type Target = MurreletString;

    fn eval_lazy(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        match self {
            LazyMurreletString::Raw(s) => Ok(MurreletString::new(s.clone())),
            LazyMurreletString::Fmt { fmt, fill } => {
                let filled = fill
                    .iter()
                    .map(|f| f.eval_lazy(ctx).map(|x| x.round() as i64))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(MurreletString::new(crate::livecode::fill_fmt(fmt, &filled)?))
            }
            LazyMurreletString::FmtFloat { fmt_f, fill } => {
                let filled = fill
                    .iter()
                    .map(|f| f.eval_lazy(ctx).map(|x| x as f64))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(MurreletString::new(crate::livecode::fill_fmt_f64(fmt_f, &filled)?))
            }
        }
    }

    fn with_more_defs(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self> {
        match self {
            LazyMurreletString::Raw(s) => Ok(LazyMurreletString::Raw(s.clone())),
            LazyMurreletString::Fmt { fmt, fill } => Ok(LazyMurreletString::Fmt {
                fmt: fmt.clone(),
                fill: fill
                    .iter()
                    .map(|f| f.with_more_defs(ctx))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
            LazyMurreletString::FmtFloat { fmt_f, fill } => Ok(LazyMurreletString::FmtFloat {
                fmt_f: fmt_f.clone(),
                fill: fill
                    .iter()
                    .map(|f| f.with_more_defs(ctx))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
        }
    }
}

pub fn eval_lazy_f32(
    v: &LazyNodeF32,
    f32min: Option<f32>,
    f32max: Option<f32>,
    ctx: &MixedEvalDefs,
) -> LivecodeResult<f32> {
    let result = match (f32min, f32max) {
        (None, None) => v.eval_lazy(ctx)?,
        (None, Some(max)) => f32::min(v.eval_lazy(ctx)?, max),
        (Some(min), None) => f32::max(min, v.eval_lazy(ctx)?),
        (Some(min), Some(max)) => f32::min(f32::max(min, v.eval_lazy(ctx)?), max),
    };
    Ok(result)
}

// can lerp between lazy items, by gathering the pairs + pct, and then evaluating them

#[derive(Clone, Debug)]
pub struct LazyLerp<T: IsLazy> {
    left: WrappedLazyType<T>,
    right: WrappedLazyType<T>,
    pct: f32, // hm, just convert to the pct...
}

impl<T: IsLazy> LazyLerp<T> {
    fn new(left: WrappedLazyType<T>, right: WrappedLazyType<T>, pct: f32) -> Self {
        Self { left, right, pct }
    }
}

// newtype to avoid orphan
#[derive(Clone, Debug)]
pub enum WrappedLazyType<T: IsLazy> {
    Single(T),
    Lerp(Box<LazyLerp<T>>),
}
impl<T> WrappedLazyType<T>
where
    T: IsLazy + std::fmt::Debug + Clone,
{
    pub(crate) fn new(x: T) -> Self {
        Self::Single(x)
    }

    pub(crate) fn new_lerp(left: WrappedLazyType<T>, right: WrappedLazyType<T>, pct: f32) -> Self {
        WrappedLazyType::Lerp(Box::new(LazyLerp::new(left, right, pct)))
    }
}

impl<T> IsLazy for LazyLerp<T>
where
    T: IsLazy,
    T::Target: Lerpable,
{
    type Target = T::Target;
    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        let left = self.left.eval_lazy(expr)?;
        let right = self.right.eval_lazy(expr)?;
        let r = left.lerpify(&right, &self.pct);

        Ok(r)
    }

    fn with_more_defs(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<Self> {
        Ok(Self {
            left: self.left.with_more_defs(more_defs)?,
            right: self.right.with_more_defs(more_defs)?,
            pct: self.pct,
        })
    }
}

impl<T> IsLazy for WrappedLazyType<T>
where
    T: IsLazy,
    T::Target: Lerpable,
{
    type Target = T::Target;
    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        match self {
            WrappedLazyType::Single(s) => s.eval_lazy(expr),
            WrappedLazyType::Lerp(s) => s.eval_lazy(expr),
        }
    }

    fn with_more_defs(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<Self> {
        Ok(match self {
            WrappedLazyType::Single(s) => WrappedLazyType::Single(s.with_more_defs(more_defs)?),
            WrappedLazyType::Lerp(s) => {
                WrappedLazyType::Lerp(Box::new(s.with_more_defs(more_defs)?))
            }
        })
    }
}

impl<T> Lerpable for WrappedLazyType<T>
where
    T: IsLazy + Clone + std::fmt::Debug,
{
    fn lerpify<M: IsLerpingMethod>(&self, other: &Self, pct: &M) -> Self {
        WrappedLazyType::new_lerp(self.clone(), other.clone(), pct.lerp_pct() as f32)
    }
}

impl<T, ControlT> LivecodeToControl<ControlT> for WrappedLazyType<T>
where
    T: LivecodeToControl<ControlT> + IsLazy,
{
    fn to_control(&self) -> ControlT {
        match self {
            WrappedLazyType::Single(inner) => inner.to_control(),
            // hax because it's just to control...
            WrappedLazyType::Lerp(lerp) => lerp.left.to_control(),
        }
    }
}
