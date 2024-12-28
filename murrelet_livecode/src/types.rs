use evalexpr::{build_operator_tree, EvalexprError, HashMapContext, Node};
use murrelet_common::IdxInRange;
use serde::{Deserialize, Deserializer};

use crate::{expr::ExprWorldContextValues, livecode::LivecodeFromWorld, state::LivecodeWorldState};

#[derive(Debug)]
pub enum LivecodeError {
    Raw(String), // my custom errors
    EvalExpr(String, EvalexprError),
    Io(String, std::io::Error),
    NestGetExtra(String),
    NestGetInvalid(String),
}
impl LivecodeError {}
impl std::fmt::Display for LivecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LivecodeError::Raw(msg) => write!(f, "{}", msg),
            LivecodeError::EvalExpr(msg, err) => write!(f, "{}: {}", msg, err),
            LivecodeError::Io(msg, err) => write!(f, "{}: {}", msg, err),
            LivecodeError::NestGetExtra(err) => {
                write!(f, "nest get has unusable tokens...: {}", err)
            }
            LivecodeError::NestGetInvalid(err) => {
                write!(f, "nest get requested for odd thing...: {}", err)
            }
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
    // #[serde(default)]
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

#[derive(Debug, Clone)]
pub enum ControlVecElement<Source> {
    Single(Source),
    Repeat(ControlVecElementRepeat<Source>),
}

impl<Source> ControlVecElement<Source> {
    pub fn raw(c: Source) -> Self {
        Self::Single(c)
    }

    pub fn eval_and_expand_vec<Target>(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>>
    where
        Source: LivecodeFromWorld<Target>,
    {
        match self {
            ControlVecElement::Single(c) => Ok(vec![c.o(w)?]),
            ControlVecElement::Repeat(r) => r.eval_and_expand_vec(w),
        }
    }
}

// chatgpt
impl<'de, Source> Deserialize<'de> for ControlVecElement<Source>
where
    Source: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_yaml::Value::deserialize(deserializer)?;

        let mut errors = Vec::new();

        // try the simple one
        match Source::deserialize(value.clone()) {
            Ok(single) => return Ok(ControlVecElement::Single(single)),
            Err(e) => errors.push(format!("Single variant failed: {}", e)),
        }

        //
        match ControlVecElementRepeat::deserialize(value.clone()) {
            Ok(repeat) => return Ok(ControlVecElement::Repeat(repeat)),
            Err(e) => {
                // it's gonna fail, so just check what
                errors.push(format!("Repeat variant failed: {}", e))
            }
        }

        // Both variants failed, return an error with detailed messages
        Err(serde::de::Error::custom(format!(
            "data did not match any variant of ControlVecElement:\n{}",
            errors.join("\n")
        )))
    }
}
