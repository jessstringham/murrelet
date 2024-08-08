use evalexpr::{build_operator_tree, EvalexprError, HashMapContext, Node};
use murrelet_common::{IdxInRange, LivecodeValue};
use serde::Deserialize;

use crate::{
    expr::{ExprWorldContextValues, MixedEvalDefs},
    livecode::LivecodeFromWorld,
    state::LivecodeWorldState,
    unitcells::EvaluableUnitCell,
};

#[derive(Debug)]
pub enum LivecodeError {
    Raw(String), // my custom errors
    EvalExpr(String, EvalexprError),
}
impl LivecodeError {}
impl std::fmt::Display for LivecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LivecodeError::Raw(msg) => write!(f, "{}", msg),
            LivecodeError::EvalExpr(msg, err) => write!(f, "{}: {}", msg, err),
        }
    }
}

impl std::error::Error for LivecodeError {}

pub type LivecodeResult<T> = Result<T, LivecodeError>;

#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct AdditionalContextNode(Node);

impl Default for AdditionalContextNode {
    fn default() -> Self {
        Self(build_operator_tree("").unwrap())
    }
}

impl AdditionalContextNode {
    pub fn eval_raw(&self, ctx: &mut HashMapContext) -> LivecodeResult<()> {
        self.0
            .eval_empty_with_context_mut(ctx)
            .map_err(|err| LivecodeError::EvalExpr("error evaluating ctx".to_owned(), err))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct LazyNodeF32Def(pub Node);

// #[derive(Debug, Deserialize, Clone)]
// #[serde(untagged)]
// pub enum LazyNodeF32Def {
//     Int(i32),
//     Bool(bool),
//     Float(f32),
//     Expr(Node),
// }

impl LazyNodeF32Def {
    pub fn new(n: Node) -> Self {
        Self(n)
    }
}

impl LivecodeFromWorld<LazyNodeF32> for LazyNodeF32Def {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(LazyNodeF32::new(self.0.clone(), w))
    }
}

impl EvaluableUnitCell<LazyNodeF32> for LazyNodeF32Def {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(LazyNodeF32::new(self.0.clone(), ctx))
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

    pub fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<f32> {
        let pct = idx.pct();
        let i = idx.i();
        let total = idx.total();

        // todo, make this more standardized
        let vs: ExprWorldContextValues = ExprWorldContextValues::new(vec![
            ("_p".to_owned(), LivecodeValue::Float(pct as f64)),
            ("_i".to_owned(), LivecodeValue::Int(i as i64)),
            ("_total".to_owned(), LivecodeValue::Int(total as i64)),
        ]);

        let mut ctx = self.world.ctx().clone();

        vs.update_ctx_with_prefix(&mut ctx, prefix);

        self.final_eval(&ctx)
    }
}

// // expr that we can add things
#[derive(Debug, Clone)]
pub enum LazyNodeF32 {
    Uninitialized,
    Node(LazyNodeF32Inner),
}

impl Default for LazyNodeF32 {
    fn default() -> Self {
        LazyNodeF32::Uninitialized
    }
}

impl LazyNodeF32 {
    pub fn new(n: Node, world: &LivecodeWorldState) -> Self {
        Self::Node(LazyNodeF32Inner {
            n,
            world: world.clone_to_lazy(),
        })
    }

    pub fn n(&self) -> Option<&Node> {
        match self {
            LazyNodeF32::Uninitialized => None,
            LazyNodeF32::Node(n) => Some(&n.n),
        }
    }

    pub fn eval_with_ctx(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<f32> {
        self.node()?.eval_with_ctx(more_defs)
    }

    pub fn final_eval(&self, ctx: &HashMapContext) -> LivecodeResult<f32> {
        self.node()?.final_eval(ctx)
    }

    pub fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<f32> {
        self.node()?.eval_idx(idx, prefix)
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
