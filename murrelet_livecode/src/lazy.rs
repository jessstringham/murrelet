use evalexpr::{HashMapContext, Node};
use murrelet_common::IdxInRange;
use serde::Deserialize;

use crate::{expr::{ExprWorldContextValues, MixedEvalDefs}, livecode::LivecodeFromWorld, state::LivecodeWorldState, types::{LivecodeError, LivecodeResult}, unitcells::EvaluableUnitCell};



#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum LazyNodeF32Def {
    Int(i32),
    Bool(bool),
    Float(f32),
    Expr(Node),
}

impl LazyNodeF32Def {
    pub fn new(n: Node) -> Self {
        Self::Expr(n)
    }

    fn result(&self) -> Result<f32, LivecodeError> {
        match self {
            LazyNodeF32Def::Int(d) => Ok(*d as f32),
            LazyNodeF32Def::Bool(d) => Ok(if *d { 1.0 } else { -1.0 }),
            LazyNodeF32Def::Float(d) => Ok(*d),
            LazyNodeF32Def::Expr(_) => Err(LivecodeError::Raw("result on a expr".to_owned())),
        }
    }
}

impl LivecodeFromWorld<LazyNodeF32> for LazyNodeF32Def {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(LazyNodeF32::new(self.clone(), w))
    }
}

impl EvaluableUnitCell<LazyNodeF32> for LazyNodeF32Def {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(LazyNodeF32::new(self.clone(), ctx))
    }
}

impl EvaluableUnitCell<LazyNodeF32> for LazyNodeF32 {
    fn eval(&self, _ctx: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(self.clone()) // ??
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
    NoCtxNode(LazyNodeF32Def),
}

impl Default for LazyNodeF32 {
    fn default() -> Self {
        LazyNodeF32::Uninitialized
    }
}

impl LazyNodeF32 {
    pub fn new(def: LazyNodeF32Def, world: &LivecodeWorldState) -> Self {
        match def {
            LazyNodeF32Def::Expr(n) => Self::Node(LazyNodeF32Inner {
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


pub trait IsLazy<Target> where
    Self: Sized {
    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<Target>;
}

impl IsLazy<f32> for LazyNodeF32 {
    fn eval_lazy(&self, expr: &MixedEvalDefs) -> LivecodeResult<f32> {
        self.eval_with_ctx(expr)
    }
} 