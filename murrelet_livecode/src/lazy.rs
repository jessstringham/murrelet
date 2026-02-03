use std::sync::Arc;

use crate::{
    expr::{ExprWorldContextValues, MixedEvalDefs, ToMixedDefs},
    livecode::{
        ControlF32, GetLivecodeIdentifiers, LivecodeFromWorld, LivecodeFunction, LivecodeToControl,
        LivecodeVariable,
    },
    nestedit::{NestEditable, NestedMod},
    state::{LivecodeWorldState, WorldWithLocalVariables},
    types::{LivecodeError, LivecodeResult},
};
use evalexpr::Node;
use glam::Vec2;
use itertools::Itertools;
use lerpable::IsLerpingMethod;
use lerpable::{step, Lerpable};
use murrelet_common::{IdxInRange, LivecodeValue, MurreletColor, MurreletIterHelpers};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlLazyNodeF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    #[cfg_attr(feature = "schemars", schemars(with = "String"))]
    Expr(Node),
}

impl ControlLazyNodeF32 {
    pub const ZERO: Self = ControlLazyNodeF32::Float(0.0);

    pub fn new(n: Node) -> Self {
        Self::Expr(n)
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
    n: Arc<Node>, // what will be evaluated!
    world: WorldWithLocalVariables, //LivecodeWorldState, // this is a reference :D
                  // more_defs: MixedEvalDefs,
}
impl LazyNodeF32Inner {
    pub fn new(n: Node, world: LivecodeWorldState) -> Self {
        Self {
            n: Arc::new(n),
            world: world.to_local(),
            // more_defs: MixedEvalDefs::new(),
        }
    }

    // options to add more details...
    pub fn add_more_defs<M: ToMixedDefs>(&self, more_defs: &M) -> Self {
        // unreachable!();
        let c = self.clone();
        c.add_expr_values(more_defs.to_mixed_def().expr_vals())

        // println!("dropping contexts...");
        // c.world
        //     .update_with_defs(MixedEvalDefsRef::new(more_defs.clone()));
        // c
    }

    pub fn add_expr_values(&self, more_vals: &ExprWorldContextValues) -> Self {
        // let mut c = self.clone();
        // c.more_defs.set_vals(more_vals);
        // c
        let mut c = self.clone();
        c.world.update_with_simple_defs(more_vals);
        c
    }

    // internal function to build the ctx
    fn build_ctx(&self) -> &WorldWithLocalVariables {
        // LivecodeResult<Arc<HashMapContext>> {
        // self.world.clone_with_mixed_defs(&self.more_defs)

        // self.world.ctx()?;

        // let w = self.world.clone();

        // let a = w.ctx().as_ref()

        // self.world.ctx()
        &self.world

        // let copied_world = self.world.ctx().as_ref().clone();
        // let mut ctx = copied_world.clone();
        // self.more_defs.update_ctx(&mut ctx)?;
        // Ok(ctx)
    }

    // what you'll use
    pub fn eval(&self) -> LivecodeResult<f32> {
        let ctx = self.build_ctx();
        // let ctx = a.as_ref();

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
            ControlLazyNodeF32::Expr(n) => Self::Node(LazyNodeF32Inner::new(n, world.clone())),
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
        let vals = ExprWorldContextValues::new_from_idx(idx).with_prefix(&format!("{}", prefix));

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

// pub fn eval_lazy_color(v: &[LazyNodeF32], ctx: &MixedEvalDefs) -> LivecodeResult<MurreletColor> {
//     Ok(murrelet_common::MurreletColor::hsva(
//         v[0].eval_lazy(ctx)?,
//         v[1].eval_lazy(ctx)?,
//         v[2].eval_lazy(ctx)?,
//         v[3].eval_lazy(ctx)?,
//     ))
// }

// pub fn eval_lazy_vec3(v: &[LazyNodeF32], ctx: &MixedEvalDefs) -> LivecodeResult<glam::Vec3> {
//     Ok(glam::vec3(
//         v[0].eval_lazy(ctx)?,
//         v[1].eval_lazy(ctx)?,
//         v[2].eval_lazy(ctx)?,
//     ))
// }

// pub fn eval_lazy_vec2(v: &[LazyNodeF32], ctx: &MixedEvalDefs) -> LivecodeResult<glam::Vec2> {
//     Ok(glam::vec2(v[0].eval_lazy(ctx)?, v[1].eval_lazy(ctx)?))
// }

#[derive(Clone, Debug, Default)]
pub struct LazyVec2 {
    x: LazyNodeF32,
    y: LazyNodeF32,
}

impl LazyVec2 {
    pub fn new(x: LazyNodeF32, y: LazyNodeF32) -> Self {
        Self { x, y }
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

#[derive(Clone, Debug, Default, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ControlLazyVec2(Vec<ControlLazyNodeF32>);
impl LivecodeFromWorld<LazyVec2> for ControlLazyVec2 {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyVec2> {
        Ok(LazyVec2::new(self.0[0].o(w)?, self.0[1].o(w)?))
    }
}

impl GetLivecodeIdentifiers for ControlLazyVec2 {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.0
            .iter()
            .map(|f| f.variable_identifiers())
            .flatten()
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.0
            .iter()
            .map(|f| f.function_identifiers())
            .flatten()
            .collect_vec()
    }
}

impl LivecodeToControl<ControlLazyVec2> for LazyVec2 {
    fn to_control(&self) -> ControlLazyVec2 {
        ControlLazyVec2(vec![self.x.to_control(), self.y.to_control()])
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

#[derive(Clone, Debug, Default, Deserialize)]
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
            .map(|f| f.variable_identifiers())
            .flatten()
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.0
            .iter()
            .map(|f| f.function_identifiers())
            .flatten()
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

#[derive(Clone, Debug, Default)]
pub struct LazyMurreletColor {
    h: LazyNodeF32,
    s: LazyNodeF32,
    v: LazyNodeF32,
    a: LazyNodeF32,
}

impl LazyMurreletColor {
    pub fn new(h: LazyNodeF32, s: LazyNodeF32, v: LazyNodeF32, a: LazyNodeF32) -> Self {
        Self { h, s, v, a }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ControlLazyMurreletColor(Vec<ControlLazyNodeF32>);
impl LivecodeFromWorld<LazyMurreletColor> for ControlLazyMurreletColor {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyMurreletColor> {
        Ok(LazyMurreletColor::new(
            self.0[0].o(w)?,
            self.0[1].o(w)?,
            self.0[2].o(w)?,
            self.0[3].o(w)?,
        ))
    }
}

impl GetLivecodeIdentifiers for ControlLazyMurreletColor {
    fn variable_identifiers(&self) -> Vec<LivecodeVariable> {
        self.0
            .iter()
            .map(|f| f.variable_identifiers())
            .flatten()
            .collect_vec()
    }

    fn function_identifiers(&self) -> Vec<LivecodeFunction> {
        self.0
            .iter()
            .map(|f| f.function_identifiers())
            .flatten()
            .collect_vec()
    }
}

impl LivecodeToControl<ControlLazyMurreletColor> for LazyMurreletColor {
    fn to_control(&self) -> ControlLazyMurreletColor {
        ControlLazyMurreletColor(vec![
            self.h.to_control(),
            self.s.to_control(),
            self.v.to_control(),
            self.a.to_control(),
        ])
    }
}

impl IsLazy for LazyMurreletColor {
    type Target = MurreletColor;

    fn eval_lazy(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        Ok(MurreletColor::hsva(
            self.h.eval_lazy(ctx)?,
            self.s.eval_lazy(ctx)?,
            self.v.eval_lazy(ctx)?,
            self.a.eval_lazy(ctx)?,
        ))
    }

    fn with_more_defs(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self> {
        Ok(LazyMurreletColor::new(
            self.h.with_more_defs(ctx)?,
            self.s.with_more_defs(ctx)?,
            self.v.with_more_defs(ctx)?,
            self.a.with_more_defs(ctx)?,
        ))
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
