use evalexpr::{HashMapContext, Node};
use murrelet_common::{IdxInRange, MurreletColor};
use serde::Deserialize;

use crate::{
    expr::{ExprWorldContextValues, MixedEvalDefs},
    livecode::LivecodeFromWorld,
    state::LivecodeWorldState,
    types::{LivecodeError, LivecodeResult},
};

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum ControlLazyNodeF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
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

// todo, figure out how to only build this context once per unitcell/etc

#[derive(Debug, Clone)]
pub struct LazyNodeF32Inner {
    n: Node,
    world: LivecodeWorldState,
}
impl LazyNodeF32Inner {
    pub fn eval_with_ctx(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<f32> {
        // start with the global ctx
        let mut ctx = self.world.clone();

        ctx.update_with_defs(more_defs)?;

        // modify it with the new data
        // todo, handle the result better
        more_defs.update_ctx(&mut ctx.ctx_mut())?;

        // now grab the actual node
        self.final_eval(&ctx.ctx())
    }

    pub fn final_eval(&self, ctx: &HashMapContext) -> LivecodeResult<f32> {
        self.n
            .eval_float_with_context(ctx)
            .map(|x| x as f32)
            .map_err(|err| LivecodeError::EvalExpr(format!("error evaluating lazy"), err))
    }

    // pub fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<f32> {
    //     let vs = ExprWorldContextValues::new_from_idx(idx, prefix);
    //     self.eval_with_expr_world_values(vs)
    // }

    pub fn eval_with_expr_world_values(&self, vs: ExprWorldContextValues) -> LivecodeResult<f32> {
        let mut ctx = self.world.ctx().clone();
        vs.update_ctx(&mut ctx)?;
        self.final_eval(&ctx)
    }
}

// // expr that we can add things
#[derive(Debug, Clone)]
pub enum LazyNodeF32 {
    Uninitialized,
    Node(LazyNodeF32Inner),
    NoCtxNode(ControlLazyNodeF32),
}

impl Default for LazyNodeF32 {
    fn default() -> Self {
        LazyNodeF32::Uninitialized
    }
}

impl LazyNodeF32 {
    pub fn new(def: ControlLazyNodeF32, world: &LivecodeWorldState) -> Self {
        match def {
            ControlLazyNodeF32::Expr(n) => Self::Node(LazyNodeF32Inner {
                n,
                world: world.clone_to_lazy(),
            }),
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
        match self {
            LazyNodeF32::Uninitialized => {
                Err(LivecodeError::Raw("uninitialized lazy node".to_owned()))
            }
            LazyNodeF32::Node(v) => v.eval_with_ctx(more_defs),
            LazyNodeF32::NoCtxNode(v) => v.result(),
        }
    }

    pub fn final_eval(&self, ctx: &HashMapContext) -> LivecodeResult<f32> {
        match self {
            LazyNodeF32::Uninitialized => {
                Err(LivecodeError::Raw("uninitialized lazy node".to_owned()))
            }
            LazyNodeF32::Node(v) => v.final_eval(ctx),
            LazyNodeF32::NoCtxNode(v) => v.result(),
        }
    }

    pub fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<f32> {
        match self {
            LazyNodeF32::Uninitialized => {
                Err(LivecodeError::Raw("uninitialized lazy node".to_owned()))
            }
            LazyNodeF32::Node(v) => {
                let vals = ExprWorldContextValues::new_from_idx(idx).with_prefix(prefix);
                v.eval_with_expr_world_values(vals)
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
}

pub trait IsLazy
where
    Self: Sized,
{
    type Target;

    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<Self::Target>;
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
