use std::{collections::HashSet, fmt::Debug, marker::PhantomData};

use evalexpr::{build_operator_tree, EvalexprError, HashMapContext, Node};
use itertools::Itertools;
use lerpable::{step, Lerpable};
use murrelet_common::IdxInRange2d;
use serde::{Deserialize, Deserializer};
use serde_yaml::Location;

use crate::{
    expr::IntoExprWorldContext,
    lazy::IsLazy,
    livecode::{GetLivecodeIdentifiers, LivecodeFromWorld},
    state::LivecodeWorldState,
    unitcells::{TmpUnitCells, UnitCellCreator, UnitCellExprWorldContext, UnitCells},
};

#[derive(Debug)]
pub enum LivecodeError {
    Raw(String), // my custom errors
    EvalExpr(String, EvalexprError),
    Io(String, std::io::Error),
    NestGetExtra(String),
    NestGetInvalid(String),
    SerdeLoc(Location, String),
    WGPU(String),
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
            LivecodeError::SerdeLoc(location, err) => {
                // if it's err, hrm, remove the controlvec ones
                let loc = format!("{},{}", location.line(), location.column());
                write!(f, "parse_error :: loc: {}, err: {}", loc, err)
            }
            LivecodeError::WGPU(err) => write!(f, "shader parse error: {}", err),
        }
    }
}

impl std::error::Error for LivecodeError {}

pub type LivecodeResult<T> = Result<T, LivecodeError>;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct AdditionalContextNode(#[cfg_attr(feature = "schemars", schemars(with = "String"))] Node);

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

impl Lerpable for AdditionalContextNode {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        step(self, other, pct)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ControlVecElementRepeatMethod {
    Single(usize),
    Rect([usize; 2]),
}
impl ControlVecElementRepeatMethod {
    fn len(&self) -> usize {
        match self {
            ControlVecElementRepeatMethod::Single(s) => *s,
            ControlVecElementRepeatMethod::Rect(r) => r[0] * r[1],
        }
    }
    fn iter(&self) -> Vec<IdxInRange2d> {
        match self {
            ControlVecElementRepeatMethod::Single(s) => {
                let mut v = vec![];
                for i in 0..*s {
                    v.push(IdxInRange2d::new(i, 1, *s));
                }
                v
            }
            ControlVecElementRepeatMethod::Rect(s) => {
                let mut v = vec![];
                for i in 0..s[0] {
                    for j in 0..s[1] {
                        v.push(IdxInRange2d::new_rect(i, j, s[0], s[1]));
                    }
                }
                v
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ControlVecElementRepeat<Source> {
    repeat: ControlVecElementRepeatMethod,
    // #[serde(default)]
    prefix: String,
    what: Vec<Source>,
}

// impl<Sequencer, Source> GetLivecodeIdentifiers for ControlVecElement<Sequencer, Source>
impl<Source> GetLivecodeIdentifiers for ControlVecElement<Source>
where
    Source: Clone + Debug + GetLivecodeIdentifiers,
    // Sequencer: UnitCellCreator + GetLivecodeIdentifiers,
{
    fn variable_identifiers(&self) -> Vec<crate::livecode::LivecodeVariable> {
        match self {
            ControlVecElement::Single(c) => c.variable_identifiers(),
            ControlVecElement::Repeat(c) => c
                .what
                .iter()
                .flat_map(|x| x.variable_identifiers())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect_vec(),
            //     ControlVecElement::UnitCell(c) => vec![
            //         c.node.variable_identifiers(),
            //         c.sequencer.variable_identifiers(),
            //     ]
            //     .concat()
            //     .into_iter()
            //     .collect::<HashSet<_>>()
            //     .into_iter()
            //     .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<crate::livecode::LivecodeFunction> {
        match self {
            ControlVecElement::Single(c) => c.function_identifiers(),
            ControlVecElement::Repeat(c) => c
                .what
                .iter()
                .flat_map(|x| x.function_identifiers())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect_vec(),
            // ControlVecElement::UnitCell(c) => vec![
            //     c.node.function_identifiers(),
            //     c.sequencer.function_identifiers(),
            // ]
            // .concat()
            // .into_iter()
            // .collect::<HashSet<_>>()
            // .into_iter()
            // .collect_vec(),
        }
    }
}

impl<Source> ControlVecElementRepeat<Source> {
    pub fn eval_and_expand_vec<Target>(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>>
    where
        Source: LivecodeFromWorld<Target>,
    {
        let mut result = Vec::with_capacity(self.repeat.len() * self.what.len());

        let prefix = if self.prefix.is_empty() {
            "i_".to_string()
        } else {
            format!("{}_", self.prefix)
        };

        for idx in self.repeat.iter() {
            let expr =
                UnitCellExprWorldContext::from_idx2d(idx, 1.0).as_expr_world_context_values();
            let new_w = w.clone_with_vals(expr, &prefix)?;

            for src in &self.what {
                let o = src.o(&new_w)?;
                result.push(o);
            }
        }
        Ok(result)
    }
}

// #[derive(Debug, Clone)]
// #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
// #[cfg_attr(
//     feature = "schemars",
//     schemars(bound = "Source: schemars::JsonSchema, ControlSequencer: schemars::JsonSchema")
// )]
// pub struct VecUnitCell<Sequencer, ControlSequencer, Source>
// where
//     Source: Clone + Debug + Default,
//     Sequencer: UnitCellCreator,
//     ControlSequencer: LivecodeFromWorld<Sequencer>,

// {
//     sequencer: ControlSequencer,
//     ctx: AdditionalContextNode,
//     prefix: String,
//     node: Source,
//     #[serde(skip)]
//     #[cfg_attr(feature = "schemars", schemars(skip))]
//     _marker: PhantomData<Sequencer>,
// }
// impl<Sequencer, ControlSequencer, Source> VecUnitCell<Sequencer, ControlSequencer, Source>
// where
//     Source: Clone + Debug + Default,
//     Sequencer: UnitCellCreator + Clone,
//     ControlSequencer: LivecodeFromWorld<Sequencer>,
// {
//     fn eval_and_expand_vec<Target>(
//         &self,
//         w: &LivecodeWorldState,
//     ) -> Result<Vec<Target>, LivecodeError>
//     where
//         Source: Clone + Debug + Default + LivecodeFromWorld<Target>,
//         Sequencer: UnitCellCreator,
//         Target: Default + Clone + Debug

//     {
//         let seq = self.sequencer.o(w)?;
//         let n: Box<dyn LivecodeFromWorld<Target>> = Box::new(self.node.clone());
//         let t = TmpUnitCells::new(
//             seq,
//             n,
//             Some(self.ctx.clone()),
//             &self.prefix,
//         ).o(w)?;
//         Ok(t.items.into_iter().map(|x| *x.node).collect_vec())
//     }
// }

#[derive(Debug, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
// pub enum ControlVecElement<Sequencer, ControlSequencer, Source>
pub enum ControlVecElement<Source>
where
    Source: Clone + Debug,
    // Sequencer: UnitCellCreator,
    // ControlSequencer: LivecodeFromWorld<Sequencer>,
{
    Single(Source),
    Repeat(ControlVecElementRepeat<Source>),
    // UnitCell(VecUnitCell<Sequencer, ControlSequencer, Source>),
}

// impl<Sequencer, ControlSequencer, Source> ControlVecElement<Sequencer, ControlSequencer, Source>
impl<Source> ControlVecElement<Source>
where
    Source: Clone + Debug,
    // Sequencer: UnitCellCreator,
    // ControlSequencer: LivecodeFromWorld<Sequencer>,
{
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
            // ControlVecElement::UnitCell(c) => c.eval_and_expand_vec(w),
        }
    }
}

// chatgpt
// impl<'de, Sequencer, ControlSequencer, Source> Deserialize<'de>
//     for ControlVecElement<Sequencer, ControlSequencer, Source>
impl<'de, Source> Deserialize<'de> for ControlVecElement<Source>
where
    Source: Deserialize<'de> + Clone + Debug,
    // Sequencer: UnitCellCreator,
    // ControlSequencer: LivecodeFromWorld<Sequencer>,
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
            Err(e) => errors.push(format!("{}", e)),
        }

        //
        match ControlVecElementRepeat::deserialize(value.clone()) {
            Ok(repeat) => return Ok(ControlVecElement::Repeat(repeat)),
            Err(e) => {
                // it's gonna fail, so just check what
                errors.push(format!("(repeat {})", e))
            }
        }

        // match VecUnitCell::deserialize(value.clone()) {
        //     Ok(repeat) => return Ok(ControlVecElement::Repeat(repeat)),
        //     Err(e) => {
        //         // it's gonna fail, so just check what
        //         errors.push(format!("(repeat {})", e))
        //     }
        // }

        // Both variants failed, return an error with detailed messages
        Err(serde::de::Error::custom(format!(
            "ControlVecElement {}",
            errors.join(" ")
        )))
    }
}
