#![allow(dead_code)]
use std::{f64::consts::PI, fmt::Debug};

use evalexpr::*;
use glam::{vec2, Vec2};
use itertools::Itertools;
use murrelet_common::{clamp, ease, lerp, map_range, smoothstep, IdxInRange, LivecodeValue};
use noise::{NoiseFn, Perlin};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::types::{AdditionalContextNode, LivecodeError, LivecodeResult};

pub fn init_evalexpr_func_ctx() -> LivecodeResult<HashMapContext> {
    context_map!{
        // constants
        "PI" => Value::Float(PI.into()),
        "ROOT2" => Value::Float(2.0_f64.sqrt()),
        "ROOT3" => Value::Float(3.0_f64.sqrt()),

        // functions
        "print" => Function::new(move |argument| {
            if let Ok(a) = argument.as_float() {
                println!("{:?} (float)", a);
            } else {
                let a = argument.as_int()?;
                println!("{:?} (int)", a);
            }
            Ok(Value::Empty)
        }),
        "manymod" => Function::new(move |argument| {
            let a = argument.as_tuple()?;

            let mut result = 0;
            let mut offset = 1;

            for val in &a {
                let tuple = val.as_fixed_len_tuple(2)?;
                let (var, mod_thing) = (tuple[0].as_number()? as i64, tuple[1].as_number()? as i64);

                result += (var % mod_thing) * offset;
                offset *= mod_thing;
            }
            Ok(Value::Int(result))
        }),

        "clamp" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (x, min, max) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let f = clamp(x as f32, min as f32, max as f32);
            Ok(Value::Float(f as f64))
        }),
        "mix" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (min, max, pct) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let f = lerp(min as f32, max as f32, pct as f32);
            Ok(Value::Float(f as f64))
        }),
        "s" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (src, out_min, out_max) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let f = map_range(src, 0.0, 1.0, out_min, out_max);
            Ok(Value::Float(f as f64))
        }),
        "s11" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (src, out_min, out_max) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let f = map_range(src, -1.0, 1.0, out_min, out_max);
            Ok(Value::Float(f as f64))
        }),
        "slog" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (src, out_min, out_max) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let f = map_range(src, 0.0, 1.0, 10.0f64.powf(out_min), 10.0f64.powf(out_max));
            Ok(Value::Float(f as f64))
        }),
        "remap" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(5)?;
            let (src, in_min, in_max, out_min, out_max) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?, tuple[3].as_number()?, tuple[4].as_number()?);
            let f = map_range(src, in_min, in_max, out_min, out_max);
            Ok(Value::Float(f as f64))
        }),
        // map and clamp. clmap.
        "clmap" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(5)?;
            let (src, in_min, in_max, out_min, out_max) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?, tuple[3].as_number()?, tuple[4].as_number()?);
            let f = map_range(clamp(src, in_min, in_max), in_min, in_max, out_min, out_max);
            Ok(Value::Float(f as f64))
        }),
        // tri(i) makes 0.5 be 1, and 0 and 1 be 0
        "tri" => Function::new(|argument| {
            let src = argument.as_number()?;
            let f = 1.0 - (src * 2.0 - 1.0).abs();
            Ok(Value::Float(f))
        }),
        // bounce(t, 0.25)
        "bounce" => Function::new(|argument| {
            let (src, mult, offset) = match argument.as_fixed_len_tuple(3) {
                Ok(tuple) => (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?),
                Err(_) => {
                    let tuple = argument.as_fixed_len_tuple(2)?;
                    (tuple[0].as_number()?, tuple[1].as_number()?, 0.0)
                }
            };
            let f = ((src * mult + offset) * PI as f64 * 2.0).sin() * 0.5 + 0.5;
            Ok(Value::Float(f))
        }),
        "saw" => Function::new(|argument| {
            let tuple = argument.as_fixed_len_tuple(2)?;
            let (src, mult) = (tuple[0].as_number()?, tuple[1].as_number()?);
            // make a sawtooth
            let f = ((src * mult) % 2.0 - 1.0).abs();
            Ok(Value::Float(f))
        }),
        "ease" => Function::new(|argument| {
            let (src, mult, offset) = match argument.as_fixed_len_tuple(3) {
                Ok(tuple) => (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?),
                Err(_) => {
                    let tuple = argument.as_fixed_len_tuple(2)?;
                    (tuple[0].as_number()?, tuple[1].as_number()?, 0.0)
                }
            };
            let f = ease(src, mult, offset);
            Ok(Value::Float(f))
        }),
        "smoothstep" => Function::new(|argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (t, edge0, edge1) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let f = smoothstep(t, edge0, edge1);
            Ok(Value::Float(f))
        }),
        "step" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(2)?;
            let (src, val) = (tuple[0].as_number()?, tuple[1].as_number()?);
            let f = if src > val { 1.0 } else { 0.0 };
            Ok(Value::Float(f as f64))
        }),
        "pulse" => Function::new(|argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (pct, t, size) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let f = smoothstep(t, pct - size, pct) - smoothstep(t, pct, pct + size);
            Ok(Value::Float(f))
        }),
        "ramp" => Function::new(|argument| {
            let tuple = argument.as_fixed_len_tuple(2)?;
            let (src, length) = (tuple[0].as_number()?, tuple[1].as_number()?);
            let f = (src * length).fract();
            Ok(Value::Float(f))
        }),
        "idx" => Function::new(|argument| {
            let tuple = argument.as_fixed_len_tuple(2)?;
            let (src, idx) = (tuple[0].as_tuple()?, tuple[1].as_number()?);
            let idx = (idx as usize) % src.len();
            let f = &src[idx];
            Ok(f.clone())
        }),
        "rn" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(2)?;
            let (seed, idx) = (tuple[0].as_number()?, tuple[1].as_number()?);
            let rn = StdRng::seed_from_u64((seed + 19247.0 * idx) as u64).gen_range(0.0..1.0);
            Ok(Value::Float(rn))
        }),
        "perlin" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(3)?;
            let (x, y, z) = (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?);
            let perlin = Perlin::new(42); // todo, should we add seed to the inputs?
            let rn = perlin.get([x, y, z]);
            Ok(Value::Float(rn))
        }),
        "len" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(2)?;
            let (x, y) = (tuple[0].as_number()?, tuple[1].as_number()?);

            let len = vec2(x as f32, y as f32).length();
            Ok(Value::Float(len as f64))
        }),

        "pow" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(2)?;
            let (x, y) = (tuple[0].as_number()?, tuple[1].as_number()?);

            let p = x.powf(y);
            Ok(Value::Float(p))
        }),
        "sin" => Function::new(move |argument| {
            let (t, w, phase) = match argument.as_fixed_len_tuple(3) {
                Ok(tuple) => (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?),
                Err(_) => {
                    match argument.as_fixed_len_tuple(2) {
                        Ok(tuple) => (tuple[0].as_number()?, tuple[1].as_number()?, 0.0),
                        Err(_) => {
                            (argument.as_float()?, 1.0, 0.0)
                        },
                    }
                }
            };
            let f = (PI * 2.0 * (w * t + phase)).sin();
            Ok(Value::Float(f as f64))
        }),
        "cos" => Function::new(move |argument| {
            let (t, w, phase) = match argument.as_fixed_len_tuple(3) {
                Ok(tuple) => (tuple[0].as_number()?, tuple[1].as_number()?, tuple[2].as_number()?),
                Err(_) => {
                    match argument.as_fixed_len_tuple(2) {
                        Ok(tuple) => (tuple[0].as_number()?, tuple[1].as_number()?, 0.0),
                        Err(_) => {
                            (argument.as_float()?, 1.0, 0.0)
                        },
                    }
                }
            };
            let f = (PI * 2.0 * (w * t + phase)).cos();
            Ok(Value::Float(f as f64))
        }),
        "res" => Function::new(move |argument| {
            let tuple = argument.as_fixed_len_tuple(9)?;
            let (x, y, aa, bb, m, n, a, b) = (
                tuple[0].as_number()?, tuple[1].as_number()?,
                tuple[2].as_number()?, tuple[3].as_number()?,
                tuple[4].as_number()?, tuple[5].as_number()?,
                tuple[6].as_number()?, tuple[7].as_number()?,
            );
            let f = aa * (m * PI * x / a).cos() * (n * PI * y / a).cos() - bb * (n * PI * x / b).cos() * (m * PI * y / b).cos();
            Ok(Value::Float(f as f64))
        })
    }.map_err(|err| {LivecodeError::EvalExpr(format!("error in init_evalexpr_func_ctx!"), err)})
}

fn lc_val_to_expr(v: &LivecodeValue) -> Value {
    match v {
        LivecodeValue::Float(f) => Value::Float(*f),
        LivecodeValue::Bool(f) => Value::Boolean(*f),
        LivecodeValue::Int(f) => Value::Int(*f),
    }
}

// simple mapping of values
#[derive(Debug, Clone)]
pub struct ExprWorldContextValues(Vec<(String, LivecodeValue)>);
impl ExprWorldContextValues {
    pub fn new(v: Vec<(String, LivecodeValue)>) -> Self {
        Self(v)
    }

    pub fn update_ctx(&self, ctx: &mut HashMapContext) -> LivecodeResult<()> {
        for (identifier, value) in &self.0 {
            // todo, maybe handle the result here to help dev
            ctx.set_value(identifier.to_owned(), lc_val_to_expr(value))
                .map_err(|err| {
                    LivecodeError::EvalExpr(format!("error setting value {}", identifier), err)
                })?;
        }
        Ok(())
    }

    pub fn set_val(&mut self, name: &str, val: LivecodeValue) {
        self.0.push((name.to_owned(), val))
    }

    pub fn new_from_idx(idx: IdxInRange) -> Self {
        Self::new(vec![
            (format!("i"), LivecodeValue::Int(idx.i() as i64)),
            (format!("if"), LivecodeValue::Float(idx.i() as f64)),
            (format!("pct"), LivecodeValue::Float(idx.pct() as f64)),
            (format!("total"), LivecodeValue::Int(idx.total() as i64)),
            (format!("totalf"), LivecodeValue::Float(idx.total() as f64)),
        ])
    }

    pub fn new_from_totaless_idx(idx: usize) -> Self {
        Self::new(vec![
            (format!("i"), LivecodeValue::Int(idx as i64)),
            (format!("if"), LivecodeValue::Float(idx as f64)),
        ])
    }

    pub fn with_prefix(&self, prefix: &str) -> Self {
        let new_vals = self
            .0
            .iter()
            .map(|(name, value)| (format!("{}{}", prefix, name), *value))
            .collect_vec();
        Self::new(new_vals)
    }

    fn combine(&mut self, vals: ExprWorldContextValues) -> Self {
        // have the new ones added later, so they'll overwrite if there are duplicates...
        Self::new([self.0.clone(), vals.0].concat())
    }
}

pub trait IntoExprWorldContext {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues;
}

impl IntoExprWorldContext for Vec<(String, f32)> {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues {
        let v = self
            .iter()
            .map(|(s, x)| (s.to_owned(), LivecodeValue::Float(*x as f64)))
            .collect_vec();
        ExprWorldContextValues(v)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GuideType {
    Horizontal,
    Diag,
}
impl GuideType {
    pub fn guides(&self) -> Vec<Vec2> {
        match self {
            GuideType::Horizontal => vec![
                // left-right
                vec2(-50.0, 0.0),
                vec2(50.0, 0.0),
                vec2(0.0, 0.0),
                // up-down
                vec2(0.0, -50.0),
                vec2(0.0, 50.0),
            ],
            GuideType::Diag => {
                vec![
                    // diag
                    vec2(50.0, -50.0),
                    vec2(-50.0, 50.0),
                    // diag
                    vec2(50.0, 50.0),
                    vec2(-50.0, -50.0),
                ]
            }
        }
    }

    pub fn border(&self) -> Vec<Vec2> {
        match self {
            GuideType::Diag => {
                vec![
                    vec2(0.0, 50.0),
                    vec2(50.0, 0.0),
                    vec2(0.0, -50.0),
                    vec2(-50.0, 0.0),
                ]
            }
            GuideType::Horizontal => {
                vec![
                    vec2(-50.0, -50.0),
                    vec2(50.0, -50.0),
                    vec2(50.0, 50.0),
                    vec2(-50.0, 50.0),
                ]
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MixedEvalDefs {
    vals: ExprWorldContextValues,
    nodes: Vec<AdditionalContextNode>, // these need to stack
}
impl MixedEvalDefs {
    pub fn new() -> Self {
        Self {
            vals: ExprWorldContextValues::new(vec![]),
            nodes: Vec::new(),
        }
    }

    pub fn new_from_expr(vals: ExprWorldContextValues) -> Self {
        Self {
            vals,
            nodes: Vec::new(),
        }
    }

    pub fn set_vals(&mut self, vals: ExprWorldContextValues) {
        self.vals = self.vals.combine(vals);
    }

    pub fn update_ctx(&self, ctx: &mut HashMapContext) -> LivecodeResult<()> {
        self.vals.update_ctx(ctx)?;
        // go from beginning to end
        for node in self.nodes.iter() {
            node.eval_raw(ctx)?;
        }

        Ok(())
    }

    pub fn set_val(&mut self, name: &str, val: LivecodeValue) {
        self.vals.set_val(name, val)
    }

    pub fn add_node(&mut self, node: AdditionalContextNode) {
        self.nodes.push(node)
    }

    pub fn combine(&self, more_defs: &MixedEvalDefs) -> Self {
        let mut c = self.clone();
        more_defs
            .nodes
            .iter()
            .for_each(|node| c.nodes.push(node.clone()));
        c.set_vals(more_defs.vals.clone());

        c
    }

    pub fn new_simple(name: &str, val: LivecodeValue) -> Self {
        let mut c = Self::new();
        c.set_val(name, val);
        c
    }
}
