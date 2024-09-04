use evalexpr::{build_operator_tree, EvalexprError, HashMapContext, Node};
use murrelet_common::IdxInRange;
use serde::Deserialize;

use crate::{
    expr::ExprWorldContextValues,
    livecode::{ControlF32, LivecodeFromWorld},
    state::LivecodeWorldState,
};

#[derive(Debug)]
pub enum LivecodeError {
    Raw(String), // my custom errors
    EvalExpr(String, EvalexprError),
    Io(String, std::io::Error),
}
impl LivecodeError {}
impl std::fmt::Display for LivecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LivecodeError::Raw(msg) => write!(f, "{}", msg),
            LivecodeError::EvalExpr(msg, err) => write!(f, "{}: {}", msg, err),
            LivecodeError::Io(msg, err) => write!(f, "{}: {}", msg, err),
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

#[derive(Debug, Clone, Deserialize)]
pub struct ControlVecElementRepeat<Source> {
    repeat: usize,
    #[serde(default)]
    prefix: String,
    what: Vec<Source>,
}

impl<Source> ControlVecElementRepeat<Source> {
    pub fn eval_and_expand_vec<Target>(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>>
    where
        Source: LivecodeFromWorld<Target>,
    {
        let mut result = Vec::with_capacity(self.repeat * self.what.len());

        let prefix = if self.prefix.is_empty() {
            "i_".to_string()
        } else {
            format!("{}_", self.prefix)
        };

        for i in 0..self.repeat {
            let idx = IdxInRange::new(i, self.repeat);
            let expr = ExprWorldContextValues::new_from_idx(idx);

            let new_w = w.clone_with_vals(expr, &prefix)?;

            for src in &self.what {
                let o = src.o(&new_w)?;
                result.push(o);
            }
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Deserialize)]
// #[serde(untagged)]
pub struct ControlVecElement<Source> {
    c: Option<Source>,
    r: Option<ControlVecElementRepeat<Source>>,
}

// i need to refactor some things now that unitcells and livecode are basically the same.
// for now just have.. copies :(
impl<Source> ControlVecElement<Source> {
    pub fn raw(c: Source) -> Self {
        Self {
            c: Some(c),
            r: None,
        }
    }

    pub fn eval_and_expand_vec<Target>(
        &self,
        w: &LivecodeWorldState,
        debug_str: &str,
    ) -> LivecodeResult<Vec<Target>>
    where
        Source: LivecodeFromWorld<Target>,
    {
        match (&self.c, &self.r) {
            (None, Some(r)) => r.eval_and_expand_vec(w),
            (Some(c), None) => Ok(vec![c.o(w)?]),
            (None, None) => Err(LivecodeError::Raw(format!(
                "vec missing both c and r {}",
                debug_str
            ))),
            (Some(_), Some(_)) => Err(LivecodeError::Raw(format!(
                "vec has both c and r {}",
                debug_str
            ))),
        }
    }
}
