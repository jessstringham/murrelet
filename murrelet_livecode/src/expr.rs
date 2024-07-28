#![allow(dead_code)]
use std::{f64::consts::PI, fmt::Debug};

use evalexpr::*;
use glam::{vec2, Vec2};
use itertools::Itertools;
use murrelet_common::{clamp, ease, lerp, map_range, print_expect, smoothstep, LivecodeValue};
use noise::{NoiseFn, Perlin};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::livecode::{LiveCodeWorldState, TimelessLiveCodeWorldState};

// livecode value to evalexpr value
fn lc_val_to_expr(v: &LivecodeValue) -> Value {
    match v {
        LivecodeValue::Float(f) => Value::Float(*f),
        LivecodeValue::Bool(f) => Value::Boolean(*f),
        LivecodeValue::Int(f) => Value::Int(*f),
    }
}

fn exec_funcs(livecode_src: Vec<(String, LivecodeValue)>) -> HashMapContext {
    let mut ctx = context_map!{

        // constants
        "PI" => Value::Float(PI.into()),
        "ROOT2" => Value::Float(2.0_f64.sqrt()),
        "ROOT3" => Value::Float(3.0_f64.sqrt()),

        // functions
        "printf" => Function::new(move |argument| {
            let a = argument.as_float()?;
            println!("{:?}", a);
            Ok(Value::Empty)
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
        })
    }.unwrap();

    for (identifier, value) in &livecode_src {
        // todo, maybe handle the result here to help dev
        ctx.set_value(identifier.to_owned(), lc_val_to_expr(value))
            .ok();
    }
    ctx
}

// sets defaults for the functions
pub fn expr_context_no_world(m: &TimelessLiveCodeWorldState) -> HashMapContext {
    let vals = m.to_timeless_vals();
    exec_funcs(vals)
}

// this is used for the global state
pub fn expr_context(w: &LiveCodeWorldState) -> HashMapContext {
    let vals = w.to_world_vals();

    let mut ctx = exec_funcs(vals);
    // when we have a world state, we'll have the global ctx, so extend that
    match w.ctx.eval_empty_with_context_mut(&mut ctx) {
        Ok(_) => (),
        Err(err) => println!("{:?}", err),
    };

    ctx
}

// simple mapping of values
pub type ExprWorldContextValues = Vec<(String, Value)>;

pub trait IntoExprWorldContext {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues;
}

impl IntoExprWorldContext for Vec<(String, f32)> {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues {
        self.iter()
            .map(|(s, x)| (s.to_owned(), Value::Float(*x as f64)))
            .collect_vec()
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

pub fn add_variable_or_prefix_it(identifier: &str, value: Value, ctx: &mut HashMapContext) {
    if ctx.get_value(identifier).is_some() {
        add_variable_or_prefix_it(&format!("_{}", identifier), value, ctx);
    } else {
        let r = ctx.set_value(identifier.to_owned(), value.clone());
        print_expect(r, "couldn't set variable");
    }
}
