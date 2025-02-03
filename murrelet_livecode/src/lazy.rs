use evalexpr::{HashMapContext, IterateVariablesContext, Node};
use itertools::Itertools;
use murrelet_common::{IdxInRange, MurreletColor};
use serde::Deserialize;

use crate::{
    expr::{ExprWorldContextValues, MixedEvalDefs},
    livecode::{GetLivecodeIdentifiers, LivecodeFromWorld, LivecodeFunction, LivecodeVariable},
    state::LivecodeWorldState,
    types::{LivecodeError, LivecodeResult},
};

#[derive(Debug, Deserialize, Clone, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ControlLazyNodeF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    #[schemars(with = "String")]
    Expr(Node),
}

impl ControlLazyNodeF32 {
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
            ControlLazyNodeF32::Expr(node) => node.iter_variable_identifiers().sorted().dedup().map(|x| LivecodeVariable::from_str(x)).collect_vec(),
            _ => vec![]
        }
    }

    fn function_identifiers(&self) -> Vec<crate::livecode::LivecodeFunction> {
        match self {
            ControlLazyNodeF32::Expr(node) => node.iter_function_identifiers().sorted().dedup().map(|x| LivecodeFunction::from_str(x)).collect_vec(),
            _ => vec![]
        }
    }
}


// todo, figure out how to only build this context once per unitcell/etc
#[derive(Debug, Clone)]
pub struct LazyNodeF32Inner {
    n: Node,                   // what will be evaluated!
    world: LivecodeWorldState, // (could be a reference...)
    more_defs: MixedEvalDefs,
}
impl LazyNodeF32Inner {
    pub fn new(n: Node, world: LivecodeWorldState) -> Self {
        Self {
            n,
            world,
            more_defs: MixedEvalDefs::new(),
        }
    }

    // options to add more details...
    pub fn add_more_defs(&self, more_defs: &MixedEvalDefs) -> Self {
        let mut c = self.clone();
        c.more_defs = c.more_defs.combine(more_defs);
        c
    }

    pub fn add_expr_values(&self, more_vals: ExprWorldContextValues) -> Self {
        let mut c = self.clone();
        c.more_defs.set_vals(more_vals);
        c
    }

    // internal function to build the ctx
    fn build_ctx(&self) -> LivecodeResult<HashMapContext> {
        let mut ctx = self.world.ctx().clone();
        self.more_defs.update_ctx(&mut ctx)?;
        Ok(ctx)
    }

    // what you'll use
    pub fn eval(&self) -> LivecodeResult<f32> {
        let ctx = self.build_ctx()?;

        self.n
            .eval_float_with_context(&ctx)
            .or_else(|_| self.n.eval_int_with_context(&ctx).map(|x| x as f64))
            .map(|x| x as f32)
            .map_err(|err| LivecodeError::EvalExpr(format!("error evaluating lazy"), err))
    }
}

// // expr that we can add things
#[derive(Debug, Clone)]
pub enum LazyNodeF32 {
    Uninitialized,
    Node(LazyNodeF32Inner),
    NoCtxNode(ControlLazyNodeF32), // this hasn't been evaluated with .o()? yet
}

impl Default for LazyNodeF32 {
    fn default() -> Self {
        LazyNodeF32::Uninitialized
    }
}

impl LazyNodeF32 {
    pub fn new(def: ControlLazyNodeF32, world: &LivecodeWorldState) -> Self {
        match def {
            ControlLazyNodeF32::Expr(n) => {
                Self::Node(LazyNodeF32Inner::new(n, world.clone_to_lazy()))
            }
            _ => Self::NoCtxNode(def),
        }
    }

    pub fn n(&self) -> Option<&Node> {
        match self {
            LazyNodeF32::Uninitialized => None,
            LazyNodeF32::Node(n) => Some(&n.n),
            LazyNodeF32::NoCtxNode(_) => None,
        }
    }

    pub fn eval_with_ctx(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<f32> {
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

    pub fn add_more_defs(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<Self> {
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
                v.add_expr_values(vals).eval()
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
            LazyNodeF32::Node(c) => Ok(c.build_ctx()?.iter_variable_names().collect_vec()),
            LazyNodeF32::NoCtxNode(_) => Err(LivecodeError::Raw("no ctx".to_owned())),
        }
    }
}

pub trait IsLazy
where
    Self: Sized,
{
    type Target;

    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<Self::Target>;

    fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<Self::Target> {
        let vals = ExprWorldContextValues::new_from_idx(idx).with_prefix(&format!("{}_", prefix));

        self.eval_lazy(&MixedEvalDefs::new_from_expr(vals))
    }
}

impl IsLazy for LazyNodeF32 {
    type Target = f32;
    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<f32> {
        self.eval_with_ctx(expr)
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

pub fn eval_lazy_color(v: &[LazyNodeF32], ctx: &MixedEvalDefs) -> LivecodeResult<MurreletColor> {
    Ok(murrelet_common::MurreletColor::hsva(
        v[0].eval_lazy(ctx)? as f32,
        v[1].eval_lazy(ctx)? as f32,
        v[2].eval_lazy(ctx)? as f32,
        v[3].eval_lazy(ctx)? as f32,
    ))
}

pub fn eval_lazy_vec3(v: &[LazyNodeF32], ctx: &MixedEvalDefs) -> LivecodeResult<glam::Vec3> {
    Ok(glam::vec3(
        v[0].eval_lazy(ctx)? as f32,
        v[1].eval_lazy(ctx)? as f32,
        v[2].eval_lazy(ctx)? as f32,
    ))
}

pub fn eval_lazy_vec2(v: &[LazyNodeF32], ctx: &MixedEvalDefs) -> LivecodeResult<glam::Vec2> {
    Ok(glam::vec2(
        v[0].eval_lazy(ctx)? as f32,
        v[1].eval_lazy(ctx)? as f32,
    ))
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
