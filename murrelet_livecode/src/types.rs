use std::{collections::HashMap, fmt};

use evalexpr::{build_operator_tree, EvalexprError, HashMapContext, Node};
use murrelet_common::{IdxInRange, LivecodeValue};
use serde::{de::{self, Visitor}, Deserialize, Deserializer};

use crate::{
    expr::{ExprWorldContextValues, MixedEvalDefs},
    livecode::{ControlF32, LivecodeFromWorld},
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
            LazyNodeF32::Node(v) => v.eval_idx(idx, prefix),
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

#[derive(Debug, Clone, Deserialize)]
pub struct ControlVecElementRepeat<Source> {
    repeat: usize,
    #[serde(default)]
    prefix: String,
    what: Vec<Source>,
}

// hack to simplify this, but should just refactor a lot
pub trait SimpleLivecode<Target> {
    fn evaluate(&self, w: &LivecodeWorldState) -> LivecodeResult<Target>;
}

impl<Src, Target> SimpleLivecode<Target> for Src
where
    Src: LivecodeFromWorld<Target>,
{
    fn evaluate(&self, w: &LivecodeWorldState) -> LivecodeResult<Target> {
        self.o(w)
    }
}

impl SimpleLivecode<f32> for ControlF32 {
    fn evaluate(&self, w: &LivecodeWorldState) -> LivecodeResult<f32> {
        self.o(w)
    }
}

impl SimpleLivecode<usize> for ControlF32 {
    fn evaluate(&self, w: &LivecodeWorldState) -> LivecodeResult<usize> {
        Ok(self.o(w)? as usize)
    }
}

impl<Source> ControlVecElementRepeat<Source> {
    pub fn eval_and_expand_vec_for_unitcell<Target>(
        &self,
        w: &LivecodeWorldState,
    ) -> LivecodeResult<Vec<Target>>
    where
        Source: EvaluableUnitCell<Target>,
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
                let o = src.eval(&new_w)?;
                result.push(o);
            }
        }
        Ok(result)
    }

    pub fn eval_and_expand_vec<Target>(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>>
    where
        Source: SimpleLivecode<Target>,
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
                let o = src.evaluate(&new_w)?;
                result.push(o);
            }
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "ik")]
pub enum ControlVecElement<Source> {
    Raw(Source),
    Repeat(ControlVecElementRepeat<Source>),
}

// custom deserializer which will
// a) keep it untagged
// b) give the error message for the Source, not the enum
// otherwise it'll just say that it doesn't match an enum variant of 
// ControlVecElement instead of what is missing
// impl<'de, Source> Deserialize<'de> for ControlVecElement<Source>
//     where Source: Deserialize<'de> {
//     fn deserialize<D>(deserializer: D) -> Result<ControlVecElement<Source>, D::Error>
//     where D: Deserializer<'de>,
//     {

//         struct ControlVecElementVisitor<Source> {
//             marker: std::marker::PhantomData<Source>,
//         }

//         impl<'de, Source> Visitor<'de> for ControlVecElementVisitor<Source>
//         where
//             Source: Deserialize<'de>,
//         {
//             type Value = ControlVecElement<Source>;

//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 formatter.write_str("a valid ControlVecElement")
//             }

//             fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
//             where
//                 V: de::MapAccess<'de>,
//             {

//                 // go through and check for a "what" field

//                 let mut has_what = false;
//                 let mut other_fields = HashMap::new();

//                 while let Some(key) = map.next_key::<String>()? {
//                     if key.as_str() == "what" {
//                         if has_what {
//                             return Err(de::Error::duplicate_field("what"));
//                         }
//                         has_what = true;
//                     } else {
//                         other_fields.insert(key, map.next_value()?);
//                     }
//                 }

//                 if has_what {

//                 }

//                 let repeat: Result<ControlVecElementRepeat<Source>, _> =
//                     Deserialize::deserialize(de::value::MapAccessDeserializer::new(&mut map));
                
//                 repeat.map(ControlVecElement::Repeat)
//                     .map_err(|err| de::Error::custom(format!("Error deserializing Repeat: {}", err)))
//             }
//         }

//         deserializer.deserialize_any(ControlVecElementVisitor {
//             marker: std::marker::PhantomData,
//         })
//     }
// }


// i need to refactor some things now that unitcells and livecode are basically the same.
// for now just have.. copies :(
impl<Source> ControlVecElement<Source> {
    pub fn eval_and_expand_vec<Target>(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>>
    where
        Source: SimpleLivecode<Target>,
    {
        match self {
            ControlVecElement::Raw(c) => Ok(vec![c.evaluate(w)?]),
            ControlVecElement::Repeat(c) => c.eval_and_expand_vec(w),
        }
    }

    pub fn eval_and_expand_vec_for_unitcell<Target>(
        &self,
        w: &LivecodeWorldState,
    ) -> LivecodeResult<Vec<Target>>
    where
        Source: EvaluableUnitCell<Target>,
    {
        match self {
            ControlVecElement::Raw(c) => Ok(vec![c.eval(w)?]),
            ControlVecElement::Repeat(c) => c.eval_and_expand_vec_for_unitcell(w),
        }
    }
}
