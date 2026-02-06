use std::{collections::HashSet, fmt::Debug};

use evalexpr::{build_operator_tree, EvalexprError, HashMapContext, Node};
use itertools::Itertools;
use lerpable::{step, Lerpable};
use murrelet_common::{print_expect, IdxInRange, IdxInRange2d, LivecodeValue};
use murrelet_gui::CanMakeGUI;
use serde::{Deserialize, Deserializer};
use thiserror::Error;

use crate::{
    expr::{IntoExprWorldContext, MixedEvalDefs, ToMixedDefs},
    lazy::{ControlLazyNodeF32, IsLazy, LazyNodeF32, WrappedLazyType},
    livecode::{
        ControlF32, GetLivecodeIdentifiers, LivecodeFromWorld, LivecodeToControl, LivecodeVariable,
    },
    state::LivecodeWorldState,
    unitcells::UnitCellIdx,
};

#[derive(Debug, Error)]
pub enum LivecodeError {
    #[error("{0}")]
    Raw(String), // my custom errors
    #[error("{0}: {1}")]
    EvalExpr(String, EvalexprError),
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    #[error("nest get requested for odd thing...: {0}")]
    NestGetExtra(String),
    #[error("nest get requested for odd thing...: {0}")]
    NestGetInvalid(String),
    #[error("parse_error :: loc: {0}, err: {1}")]
    SerdeLoc(String, String),
    #[error("shader parse error: {0}")]
    WGPU(String),
    #[error("parse: {0}")]
    JsonParse(String),
}
impl LivecodeError {
    pub fn raw(s: &str) -> Self {
        Self::Raw(s.to_string())
    }

    pub fn rawr<T>(s: &str) -> LivecodeResult<T> {
        LivecodeResult::Err(Self::raw(s))
    }
}

pub trait IterUnwrapOrPrint<T> {
    fn iter_unwrap<U, F>(&self, err: &str, f: F) -> Vec<U>
    where
        F: Fn(&T) -> LivecodeResult<U>;
}

impl<T> IterUnwrapOrPrint<T> for Vec<T> {
    fn iter_unwrap<U, F>(&self, err: &str, f: F) -> Vec<U>
    where
        F: Fn(&T) -> LivecodeResult<U>,
    {
        let res: LivecodeResult<Vec<U>> = self.iter().map(|d| f(d)).collect();
        print_expect(res, err).unwrap_or(vec![])
    }
}

// impl std::fmt::Display for LivecodeError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             LivecodeError::Raw(msg) => write!(f, "{}", msg),
//             LivecodeError::EvalExpr(msg, err) => write!(f, "{}: {}", msg, err),
//             LivecodeError::Io(msg, err) => write!(f, "{}: {}", msg, err),
//             LivecodeError::NestGetExtra(err) => {
//                 write!(f, "nest get has unusable tokens...: {}", err)
//             }
//             LivecodeError::NestGetInvalid(err) => {
//                 write!(f, "nest get requested for odd thing...: {}", err)
//             }
//             LivecodeError::SerdeLoc(location, err) => {
//                 // if it's err, hrm, remove the controlvec ones
//                 let loc = format!("{},{}", location.line(), location.column());
//                 write!(f, "parse_error :: loc: {},{}, err: {}", loc, err)
//             }
//             LivecodeError::WGPU(err) => write!(f, "shader parse error: {}", err),
//         }
//     }
// }

// impl std::error::Error for LivecodeError {}

pub type LivecodeResult<T> = Result<T, LivecodeError>;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct AdditionalContextNode(#[cfg_attr(feature = "schemars", schemars(with = "String"))] Node);

fn _default_ctx() -> AdditionalContextNode {
    AdditionalContextNode::new_dummy()
}

fn _default_ctx_lazy() -> AdditionalContextNode {
    AdditionalContextNode::new_dummy()
}

impl Default for AdditionalContextNode {
    fn default() -> Self {
        Self(build_operator_tree("").unwrap())
    }
}

impl AdditionalContextNode {
    pub fn vars(&self) -> Vec<LivecodeVariable> {
        self.0
            .iter_variable_identifiers()
            .sorted()
            .dedup()
            .map(LivecodeVariable::from_str)
            .collect_vec()
    }

    pub fn eval_raw(&self, ctx: &mut HashMapContext) -> LivecodeResult<()> {
        self.0
            .eval_empty_with_context_mut(ctx)
            .map_err(|err| LivecodeError::EvalExpr("error evaluating ctx".to_owned(), err))
    }

    pub fn new_dummy() -> AdditionalContextNode {
        AdditionalContextNode(build_operator_tree("").unwrap())
    }
}

impl CanMakeGUI for AdditionalContextNode {
    fn make_gui() -> murrelet_gui::MurreletGUISchema {
        murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Defs)
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
    Single(ControlF32),
    Rect([ControlF32; 2]),
}
impl ControlVecElementRepeatMethod {
    fn len(&self, w: &LivecodeWorldState) -> LivecodeResult<usize> {
        let v = match self {
            ControlVecElementRepeatMethod::Single(s) => {
                let ss = s.o(w)?;
                ss
            }
            ControlVecElementRepeatMethod::Rect(r) => {
                let rr = r.o(w)?;
                rr.x * rr.y
            }
        };
        Ok(v as usize)
    }
    fn iter(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<IdxInRange2d>> {
        let v = match self {
            ControlVecElementRepeatMethod::Single(s) => {
                IdxInRange::enumerate_count(s.o(w)? as usize)
                    .iter()
                    .map(|x| x.to_2d())
                    .collect_vec()
            }
            ControlVecElementRepeatMethod::Rect(s) => {
                let rr = s.o(w)?;
                IdxInRange2d::enumerate_counts(rr.x as usize, rr.y as usize)
            }
        };
        Ok(v)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ControlLazyBlendRepeatMethod {
    count: ControlLazyNodeF32,
    blend: ControlLazyNodeF32,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum DeserLazyControlVecElementRepeatMethod {
    Blend(ControlLazyBlendRepeatMethod),
    Single(ControlLazyNodeF32),
    Rect([ControlLazyNodeF32; 2]),
}
impl DeserLazyControlVecElementRepeatMethod {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyVecElementRepeatMethod> {
        match self {
            DeserLazyControlVecElementRepeatMethod::Single(lazy) => {
                let v = lazy.o(w)?;
                Ok(LazyVecElementRepeatMethod::Single(v))
            }
            DeserLazyControlVecElementRepeatMethod::Rect(lazy) => {
                let x = lazy[0].o(w)?;
                let y = lazy[1].o(w)?;
                Ok(LazyVecElementRepeatMethod::Rect([x, y]))
            }
            DeserLazyControlVecElementRepeatMethod::Blend(b) => {
                let count = b.count.o(w)?;
                let blend = b.blend.o(w)?;

                Ok(LazyVecElementRepeatMethod::Blend(LazyBlendRepeatMethod {
                    count,
                    blend,
                }))
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DeserLazyControlVecElementRepeat<DeserSource: Clone + Debug> {
    repeat: DeserLazyControlVecElementRepeatMethod,
    #[serde(default = "_default_ctx")]
    ctx: AdditionalContextNode,
    prefix: String,
    what: Vec<DeserLazyControlVecElement<DeserSource>>,
}
impl<DeserSource: Clone + Debug> DeserLazyControlVecElementRepeat<DeserSource> {
    fn o<LazySource, Target>(
        &self,
        w: &LivecodeWorldState,
    ) -> LivecodeResult<LazyVecElementRepeat<WrappedLazyType<LazySource>>>
    where
        DeserSource: LivecodeFromWorld<LazySource>,
        LazySource: IsLazy<Target = Target> + Debug + Clone,
        Target: Lerpable,
    {
        let what = self
            .what
            .iter()
            .map(|x| x.o::<LazySource, Target>(w))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(LazyVecElementRepeat {
            repeat: self.repeat.o(w)?,
            ctx: self.ctx.clone(),
            prefix: self.prefix.clone(),
            what,
        })
    }
}

#[derive(Debug, Clone)]
pub enum DeserLazyControlVecElement<Source>
where
    Source: Clone + Debug,
{
    Single(Source),
    Repeat(DeserLazyControlVecElementRepeat<Source>),
}

impl<Source> DeserLazyControlVecElement<Source>
where
    Source: Clone + Debug,
{
    pub fn raw(c: Source) -> Self {
        Self::Single(c)
    }
}
impl<DeserSource: Debug + Clone> DeserLazyControlVecElement<DeserSource> {
    pub fn o<LazySource, Target>(
        &self,
        w: &LivecodeWorldState,
    ) -> LivecodeResult<LazyControlVecElement<WrappedLazyType<LazySource>>>
    where
        DeserSource: LivecodeFromWorld<LazySource>,
        LazySource: Debug + Clone + IsLazy<Target = Target>,
        Target: Lerpable,
    {
        let a = match self {
            DeserLazyControlVecElement::Single(a) => {
                LazyControlVecElement::Single(WrappedLazyType::new(a.o(w)?))
            }
            DeserLazyControlVecElement::Repeat(r) => {
                LazyControlVecElement::Repeat(r.o::<LazySource, Target>(w)?)
            }
        };
        Ok(a)
    }
}

// chatgpt
impl<'de, Source> Deserialize<'de> for DeserLazyControlVecElement<Source>
where
    Source: Deserialize<'de> + Clone + Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_yaml::Value::deserialize(deserializer)?;

        let mut errors = Vec::new();

        // try the simple one
        match Source::deserialize(value.clone()) {
            Ok(single) => return Ok(DeserLazyControlVecElement::Single(single)),
            Err(e) => errors.push(format!("{}", e)),
        }

        match DeserLazyControlVecElementRepeat::deserialize(value.clone()) {
            Ok(repeat) => return Ok(DeserLazyControlVecElement::Repeat(repeat)),
            Err(e) => {
                // it's gonna fail, so just check what
                errors.push(format!("(repeat {})", e))
            }
        }

        // Both variants failed, return an error with detailed messages
        Err(serde::de::Error::custom(format!(
            "ControlVecElement {}",
            errors.join(" ")
        )))
    }
}

#[cfg(feature = "schemars")]
impl<Source> schemars::JsonSchema for DeserLazyControlVecElement<Source>
where
    Source: schemars::JsonSchema + Clone + Debug,
{
    fn schema_name() -> String {
        format!("LazyControlVecElement_{}", Source::schema_name())
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{Schema, SchemaObject, SubschemaValidation};
        // Variant 1: plain Source (your Single case without a wrapper key)
        let single_schema = Source::json_schema(gen);
        // Variant 2: the repeat object
        let repeat_schema = <DeserLazyControlVecElementRepeat<Source>>::json_schema(gen);

        Schema::Object(SchemaObject {
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![single_schema, repeat_schema]),
                ..Default::default()
            })),
            metadata: Some(Box::new(schemars::schema::Metadata {
                description: Some(
                    "Either a single element (inline) OR a repeat object { repeat, prefix?, what }"
                        .to_string(),
                ),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone)]
pub struct LazyBlendRepeatMethod {
    count: LazyNodeF32,
    blend: LazyNodeF32,
}

// just an intermediate type...
#[derive(Debug, Clone)]
pub enum LazyVecElementRepeatMethod {
    Blend(LazyBlendRepeatMethod),
    Single(LazyNodeF32),
    Rect([LazyNodeF32; 2]),
}
impl LazyVecElementRepeatMethod {
    fn len(&self, ctx: &MixedEvalDefs) -> LivecodeResult<usize> {
        let v = match self {
            LazyVecElementRepeatMethod::Single(s) => {
                let ss = s.eval_lazy(ctx)?;
                ss
            }
            LazyVecElementRepeatMethod::Rect(r) => {
                let rx = r[0].eval_lazy(ctx)?;
                let ry = r[1].eval_lazy(ctx)?;
                rx * ry
            }
            LazyVecElementRepeatMethod::Blend(b) => {
                let ss = b.count.eval_lazy(ctx)?;
                ss
            }
        };
        Ok(v as usize)
    }
    fn iter(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Vec<IdxInRange2d>> {
        let v = match self {
            LazyVecElementRepeatMethod::Single(s) => {
                IdxInRange::enumerate_count(s.eval_lazy(ctx)? as usize)
                    .iter()
                    .map(|x| x.to_2d())
                    .collect_vec()
            }
            LazyVecElementRepeatMethod::Rect(s) => {
                let rx = s[0].eval_lazy(ctx)?;
                let ry = s[1].eval_lazy(ctx)?;
                IdxInRange2d::enumerate_counts(rx as usize, ry as usize)
            }
            LazyVecElementRepeatMethod::Blend(b) => IdxInRange::enumerate_count(
                b.count.eval_lazy(ctx)? as usize + b.blend.eval_lazy(ctx)? as usize,
            )
            .iter()
            .map(|x| x.to_2d())
            .collect_vec(),
        };
        Ok(v)
    }

    fn next_blend(&self, ctx: &MixedEvalDefs) -> Option<BlendWith> {
        match self {
            LazyVecElementRepeatMethod::Blend(b) => {
                let blend = b.blend.eval_with_ctx(ctx).unwrap_or_default() as usize;
                if blend > 0 {
                    Some(BlendWith::new(blend))
                } else {
                    None
                }
            }
            LazyVecElementRepeatMethod::Single(_) => None,
            LazyVecElementRepeatMethod::Rect(_) => None,
        }
    }
}

// just internal method, if we realize we're looking at a lazy,
#[derive(Debug, Clone)]
pub struct LazyVecElementRepeat<Source: Clone + Debug + IsLazy> {
    repeat: LazyVecElementRepeatMethod,
    ctx: AdditionalContextNode,
    prefix: String,
    what: Vec<LazyControlVecElement<Source>>,
}

impl<Inner> LazyVecElementRepeat<WrappedLazyType<Inner>>
where
    Inner: Clone + Debug + IsLazy,
    Inner::Target: Lerpable,
{
    // expands repeats + applies defs, while preserving WrappedLazyType nodes for blending
    pub fn lazy_expand_vec_repeat_element(
        &self,
        ctx: &MixedEvalDefs,
    ) -> LivecodeResult<Vec<WrappedLazyType<Inner>>>
    where
        Inner::Target: Lerpable,
    {
        let mut result: Vec<WrappedLazyType<Inner>> =
            Vec::with_capacity(self.repeat.len(ctx)? * self.what.len());

        let prefix = if self.prefix.is_empty() {
            "i_".to_string()
        } else {
            format!("{}_", self.prefix)
        };

        for idx in self.repeat.iter(ctx)? {
            let mut scoped_ctx = ctx.clone();
            scoped_ctx.add_node(self.ctx.clone());
            let expr = UnitCellIdx::from_idx2d(idx, 1.0).as_expr_world_context_values();
            scoped_ctx.set_vals(expr.with_prefix(&prefix));

            let mut is_blending: Option<BlendWith> = None;

            for src in &self.what {
                match src {
                    LazyControlVecElement::Single(c) => {
                        let item = c.with_more_defs(&scoped_ctx)?;
                        blend_with_list(&mut result, item, &mut is_blending);
                    }
                    LazyControlVecElement::Repeat(c) => {
                        let nested = c.lazy_expand_vec_repeat_element(&scoped_ctx)?;
                        for item in nested {
                            blend_with_list(&mut result, item, &mut is_blending);
                        }

                        if let Some(new_blend) = c.repeat.next_blend(&scoped_ctx) {
                            is_blending = Some(new_blend);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn with_more_defs(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self>
    where
        Inner::Target: Lerpable,
    {
        Ok(Self {
            repeat: self.repeat.clone(),
            ctx: self.ctx.clone(),
            prefix: self.prefix.clone(),
            what: self
                .what
                .iter()
                .map(|elem| elem.with_more_defs(ctx))
                .collect::<LivecodeResult<Vec<_>>>()?,
        })
    }
}

impl<Source, ControlSource> LivecodeToControl<DeserLazyControlVecElement<ControlSource>>
    for LazyControlVecElement<Source>
where
    Source: Debug + Clone + LivecodeToControl<ControlSource> + IsLazy,
    ControlSource: Debug + Clone,
{
    fn to_control(&self) -> DeserLazyControlVecElement<ControlSource> {
        match self {
            LazyControlVecElement::Single(src) => {
                DeserLazyControlVecElement::Single(src.to_control())
            }
            LazyControlVecElement::Repeat(rep) => {
                let repeat = match &rep.repeat {
                    LazyVecElementRepeatMethod::Single(l) => {
                        DeserLazyControlVecElementRepeatMethod::Single(l.to_control())
                    }
                    LazyVecElementRepeatMethod::Rect([x, y]) => {
                        DeserLazyControlVecElementRepeatMethod::Rect([
                            x.to_control(),
                            y.to_control(),
                        ])
                    }
                    LazyVecElementRepeatMethod::Blend(b) => {
                        DeserLazyControlVecElementRepeatMethod::Blend(
                            ControlLazyBlendRepeatMethod {
                                count: b.count.to_control(),
                                blend: b.blend.to_control(),
                            },
                        )
                    }
                };

                let what = rep.what.iter().map(|e| e.to_control()).collect::<Vec<_>>();

                DeserLazyControlVecElement::Repeat(DeserLazyControlVecElementRepeat {
                    repeat,
                    prefix: rep.prefix.clone(),
                    what,
                    ctx: rep.ctx.clone(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ControlVecElementRepeat<Source: Clone + Debug> {
    repeat: ControlVecElementRepeatMethod,
    // #[serde(default)]
    prefix: String,
    what: Vec<ControlVecElement<Source>>,
    #[serde(default)]
    blend_with_next: usize,
}

// impl<Sequencer, Source> GetLivecodeIdentifiers for ControlVecElement<Sequencer, Source>
impl<Source: Clone + Debug> GetLivecodeIdentifiers for ControlVecElement<Source>
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
        }
    }
}

// chatgpt wanted to use this instead of IdxInRange, but I can probably switch back to idxinrange...
#[derive(Debug, Clone, Copy)]
struct BlendWith {
    offset: usize, // 0 = last item
    count: usize,  // number of items to blend
}

impl BlendWith {
    fn new(count: usize) -> Self {
        assert!(count > 0, "blend_with_next requires count > 0");
        Self {
            offset: count - 1,
            count,
        }
    }

    fn pct(&self) -> f32 {
        (self.offset + 1) as f32 / (self.count + 1) as f32
    }

    fn prev(self) -> Option<Self> {
        if self.offset == 0 {
            None
        } else {
            Some(Self {
                offset: self.offset - 1,
                count: self.count,
            })
        }
    }
}

fn blend_with_list<Target: Lerpable>(
    result: &mut Vec<Target>,
    new: Target,
    is_blending: &mut Option<BlendWith>,
) {
    if let Some(curr_offset) = is_blending {
        // otherwise, too far back, skip
        if curr_offset.offset < result.len() {
            let i = result.len() - 1 - curr_offset.offset;

            result[i] = new.lerpify(&result[i], &curr_offset.pct());

            // if we can subtract 1, that's the next one to check.
            // otherwise, set to none.
            *is_blending = curr_offset.prev();
        }
    } else {
        result.push(new);
    }
}

pub fn eval_and_expand_vec_list<Source, Target>(
    items: &[ControlVecElement<Source>],
    w: &LivecodeWorldState,
) -> LivecodeResult<Vec<Target>>
where
    Source: LivecodeFromWorld<Target> + Clone + Debug,
    Target: Lerpable,
{
    let mut result: Vec<Target> = Vec::new();
    let mut is_blending: Option<BlendWith> = None;

    for item in items {
        let expanded = item.eval_and_expand_vec(w)?;
        for elem in expanded {
            blend_with_list(&mut result, elem, &mut is_blending);
        }

        let blend_count = item.blend_with_next();
        if blend_count > 0 {
            is_blending = Some(BlendWith::new(blend_count));
        }
    }

    Ok(result)
}

pub fn lazy_expand_vec_list<Inner>(
    items: &[LazyControlVecElement<WrappedLazyType<Inner>>],
    ctx: &MixedEvalDefs,
) -> LivecodeResult<Vec<WrappedLazyType<Inner>>>
where
    Inner: Clone + Debug + IsLazy,
    Inner::Target: Lerpable,
{
    let mut result: Vec<WrappedLazyType<Inner>> = Vec::new();
    let mut is_blending: Option<BlendWith> = None;

    for item in items {
        let expanded = item.lazy_expand_vec(ctx)?;
        for elem in expanded {
            blend_with_list(&mut result, elem, &mut is_blending);
        }

        if let Some(blend_count) = item.blend_with_next(ctx) {
            is_blending = Some(blend_count);
        }
    }

    Ok(result)
}

impl<Source: Clone + Debug> ControlVecElementRepeat<Source> {
    pub fn _eval_and_expand_vec<Target>(
        &self,
        w: &LivecodeWorldState,
        offset: usize,
    ) -> LivecodeResult<(usize, Vec<Target>)>
    where
        Source: LivecodeFromWorld<Target>,
        Target: Lerpable,
    {
        let mut result = Vec::with_capacity(self.repeat.len(w)? * self.what.len());

        let prefix = if self.prefix.is_empty() {
            "i_".to_string()
        } else {
            format!("{}_", self.prefix)
        };

        let mut offset = offset;

        for idx in self.repeat.iter(w)? {
            let expr = UnitCellIdx::from_idx2d(idx, 1.0).as_expr_world_context_values();
            let mut new_w = w.clone_with_vals(expr, &prefix);

            let mut is_blending: Option<BlendWith> = None;

            for src in &self.what {
                match src {
                    ControlVecElement::Single(c) => {
                        // just update it and overwrite it...
                        new_w.update_with_defs(
                            ("vseed", LivecodeValue::float(offset as f32)).to_mixed_def(),
                        );
                        let o = c.o(&new_w)?;

                        blend_with_list(&mut result, o, &mut is_blending);

                        offset += 1;
                    }
                    ControlVecElement::Repeat(c) => {
                        let (new_offset, o) = c._eval_and_expand_vec(&new_w, offset)?;

                        for item in o.into_iter() {
                            blend_with_list(&mut result, item, &mut is_blending);
                        }

                        if c.blend_with_next > 0 {
                            is_blending = Some(BlendWith::new(c.blend_with_next));
                        }

                        offset = new_offset;
                    }
                }
            }
        }
        Ok((offset, result))
    }

    pub fn eval_and_expand_vec<Target>(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>>
    where
        Source: LivecodeFromWorld<Target>,
        Target: Lerpable,
    {
        let (_, a) = self._eval_and_expand_vec(w, 0)?;
        Ok(a)
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
// #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
// pub enum ControlVecElement<Sequencer, ControlSequencer, Source>
pub enum ControlVecElement<Source>
where
    Source: Clone + Debug,
{
    Single(Source),
    Repeat(ControlVecElementRepeat<Source>),
    // UnitCell(VecUnitCell<Sequencer, ControlSequencer, Source>),
}

#[derive(Debug, Clone)]
pub enum LazyControlVecElement<Source>
where
    Source: Clone + Debug + crate::lazy::IsLazy,
{
    Single(Source),
    Repeat(LazyVecElementRepeat<Source>),
}

impl<Inner> IsLazy for LazyControlVecElement<WrappedLazyType<Inner>>
where
    Inner: Clone + Debug + IsLazy,
    Inner::Target: Lerpable,
{
    type Target = Vec<Inner::Target>;

    fn eval_lazy(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self::Target> {
        let expanded: Vec<WrappedLazyType<Inner>> = self.lazy_expand_vec(ctx)?;
        expanded.into_iter().map(|s| s.eval_lazy(ctx)).collect()
    }

    fn with_more_defs(&self, ctx: &MixedEvalDefs) -> LivecodeResult<Self> {
        Ok(match self {
            LazyControlVecElement::Single(s) => {
                LazyControlVecElement::Single(s.with_more_defs(ctx)?)
            }
            LazyControlVecElement::Repeat(rep) => {
                LazyControlVecElement::Repeat(rep.with_more_defs(ctx)?)
            }
        })
    }
}

impl<Inner> LazyControlVecElement<WrappedLazyType<Inner>>
where
    Inner: Clone + Debug + IsLazy,
    Inner::Target: Lerpable,
{
    pub fn lazy_expand_vec(
        &self,
        ctx: &MixedEvalDefs,
    ) -> LivecodeResult<Vec<WrappedLazyType<Inner>>> {
        match self {
            LazyControlVecElement::Single(c) => Ok(vec![c.clone()]),
            LazyControlVecElement::Repeat(c) => c.lazy_expand_vec_repeat_element(ctx),
        }
    }

    fn blend_with_next(&self, ctx: &MixedEvalDefs) -> Option<BlendWith> {
        match self {
            LazyControlVecElement::Single(_) => None,
            LazyControlVecElement::Repeat(r) => r.repeat.next_blend(ctx),
        }
    }
}

impl<Source> ControlVecElement<Source>
where
    Source: Clone + Debug,
    // Sequencer: UnitCellCreator,
    // ControlSequencer: LivecodeFromWorld<Sequencer>,
{
    fn blend_with_next(&self) -> usize {
        match self {
            ControlVecElement::Single(_) => 0,
            ControlVecElement::Repeat(r) => r.blend_with_next,
        }
    }

    pub fn raw(c: Source) -> Self {
        Self::Single(c)
    }

    pub fn eval_and_expand_vec<Target>(&self, w: &LivecodeWorldState) -> LivecodeResult<Vec<Target>>
    where
        Source: LivecodeFromWorld<Target>,
        Target: Lerpable,
    {
        match self {
            ControlVecElement::Single(c) => Ok(vec![c.o(w)?]),
            ControlVecElement::Repeat(r) => r.eval_and_expand_vec(w),
            // ControlVecElement::UnitCell(c) => c.eval_and_expand_vec(w),
        }
    }
}

// chatgpt can implement my deserializers...

#[cfg(feature = "schemars")]
impl<Source> schemars::JsonSchema for ControlVecElement<Source>
where
    Source: schemars::JsonSchema + Clone + Debug,
{
    fn schema_name() -> String {
        format!("ControlVecElement_{}", Source::schema_name())
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{Schema, SchemaObject, SubschemaValidation};
        // Variant 1: plain Source (your Single case without a wrapper key)
        let single_schema = Source::json_schema(gen);
        // Variant 2: the repeat object
        let repeat_schema = <ControlVecElementRepeat<Source>>::json_schema(gen);

        Schema::Object(SchemaObject {
            subschemas: Some(Box::new(SubschemaValidation {
                one_of: Some(vec![single_schema, repeat_schema]),
                ..Default::default()
            })),
            metadata: Some(Box::new(schemars::schema::Metadata {
                description: Some(
                    "Either a single element (inline) OR a repeat object { repeat, prefix?, what }"
                        .to_string(),
                ),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}

impl<Source> GetLivecodeIdentifiers for DeserLazyControlVecElement<Source>
where
    Source: Clone + Debug + GetLivecodeIdentifiers,
{
    fn variable_identifiers(&self) -> Vec<crate::livecode::LivecodeVariable> {
        match self {
            DeserLazyControlVecElement::Single(c) => c.variable_identifiers(),
            DeserLazyControlVecElement::Repeat(r) => r
                .what
                .iter()
                .flat_map(|x| x.variable_identifiers())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect_vec(),
        }
    }

    fn function_identifiers(&self) -> Vec<crate::livecode::LivecodeFunction> {
        match self {
            DeserLazyControlVecElement::Single(c) => c.function_identifiers(),
            DeserLazyControlVecElement::Repeat(r) => r
                .what
                .iter()
                .flat_map(|x| x.function_identifiers())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect_vec(),
        }
    }
}

// chatgpt can implement my deserializers...
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

        match ControlVecElementRepeat::deserialize(value.clone()) {
            Ok(repeat) => return Ok(ControlVecElement::Repeat(repeat)),
            Err(e) => {
                // it's gonna fail, so just check what
                errors.push(format!("(repeat {})", e))
            }
        }

        // Both variants failed, return an error with detailed messages
        Err(serde::de::Error::custom(format!(
            "ControlVecElement {}",
            errors.join(" ")
        )))
    }
}

impl<LazyTarget: Debug + Clone> crate::nestedit::NestEditable for LazyControlVecElement<LazyTarget>
where
    LazyTarget: IsLazy,
{
    fn nest_update(&self, _mods: crate::nestedit::NestedMod) -> Self {
        self.clone()
    }

    fn nest_get(&self, _getter: &[&str]) -> LivecodeResult<String> {
        Err(LivecodeError::NestGetExtra(
            "LazyControlVecElement".to_owned(),
        ))
    }
}
