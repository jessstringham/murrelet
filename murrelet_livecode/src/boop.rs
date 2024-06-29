#![allow(dead_code)]
use std::{collections::HashMap, f32::consts::PI};

use glam::{vec2, vec3, Vec2, Vec3};
use itertools::Itertools;
use murrelet_common::MurreletColor;

// can turn a target T into Self and back
pub trait BoopFromWorld<T>
where
    Self: Sized,
{
    fn boop_init(conf: &BoopConf, target: &T) -> Self {
        Self::boop_init_at_time(conf, 0.0, target)
    }

    fn boop_init_at_time(conf: &BoopConf, t: f32, target: &T) -> Self;

    fn boop(&mut self, conf: &BoopConf, t: f32, target: &T) -> T;

    fn any_weird_states(&self) -> bool;
}

// ugh, like this one takes in the BoopInnerConf
pub trait PrimitiveBoopFromWorld<T>
where
    Self: Sized,
{
    fn boop_init(conf: &BoopConf, target: &T) -> Self {
        Self::boop_init_at_time(conf, 0.0, target)
    }

    fn boop_init_at_time(conf: &BoopConf, t: f32, target: &T) -> Self;

    fn boop(&mut self, conf: &BoopConf, t: f32, target: &T) -> T;

    fn any_weird_states(&self) -> bool;
}

pub type BoopState2 = [BoopState; 2];

impl BoopFromWorld<Vec2> for BoopState2 {
    fn boop(&mut self, conf: &BoopConf, t: f32, target: &Vec2) -> Vec2 {
        vec2(
            self[0].boop(conf, t, &target.x),
            self[1].boop(conf, t, &target.y),
        )
    }

    fn boop_init_at_time(conf: &BoopConf, t: f32, target: &Vec2) -> Self {
        [
            BoopState::boop_init_at_time(conf, t, &target.x),
            BoopState::boop_init_at_time(conf, t, &target.y),
        ]
    }

    fn any_weird_states(&self) -> bool {
        self[0].is_weird_state() || self[1].is_weird_state()
    }
}

pub type BoopState3 = [BoopState; 3];

impl BoopFromWorld<Vec3> for BoopState3 {
    fn boop(&mut self, conf: &BoopConf, t: f32, target: &Vec3) -> Vec3 {
        vec3(
            self[0].boop(conf, t, &target.x),
            self[1].boop(conf, t, &target.y),
            self[2].boop(conf, t, &target.z),
        )
    }

    fn boop_init_at_time(conf: &BoopConf, t: f32, target: &Vec3) -> Self {
        [
            BoopState::boop_init_at_time(conf, t, &target.x),
            BoopState::boop_init_at_time(conf, t, &target.y),
            BoopState::boop_init_at_time(conf, t, &target.z),
        ]
    }

    fn any_weird_states(&self) -> bool {
        self[0].is_weird_state() || self[1].is_weird_state() || self[2].is_weird_state()
    }
}

pub type BoopStateHsva = [BoopState; 4];

impl BoopFromWorld<MurreletColor> for BoopStateHsva {
    fn boop(&mut self, conf: &BoopConf, t: f32, target: &MurreletColor) -> MurreletColor {
        // todo, uh, how do i want to do this, and no clue if i'm doing this right

        let h = target.into_hsva_components();

        MurreletColor::hsva(
            self[0].boop(conf, t, &h[0]),
            self[1].boop(conf, t, &h[1]),
            self[2].boop(conf, t, &h[2]),
            self[2].boop(conf, t, &h[3]),
        )
    }

    fn boop_init_at_time(conf: &BoopConf, t: f32, target: &MurreletColor) -> Self {
        let h = target.into_hsva_components();
        [
            BoopState::boop_init_at_time(conf, t, &h[0]),
            BoopState::boop_init_at_time(conf, t, &h[1]),
            BoopState::boop_init_at_time(conf, t, &h[2]),
            BoopState::boop_init_at_time(conf, t, &h[3]),
        ]
    }

    fn any_weird_states(&self) -> bool {
        self[0].is_weird_state()
            || self[1].is_weird_state()
            || self[2].is_weird_state()
            || self[3].is_weird_state()
    }
}

impl BoopFromWorld<f32> for BoopState {
    fn boop(&mut self, conf: &BoopConf, t: f32, target: &f32) -> f32 {
        let (maybe_new, result) = self._boop(conf, t, *target);

        if let Some(new) = maybe_new {
            *self = new;
        }

        result
    }

    fn boop_init_at_time(conf: &BoopConf, t: f32, target: &f32) -> Self {
        BoopState::new(conf, t, *target)
    }

    fn any_weird_states(&self) -> bool {
        self.is_weird_state()
    }
}

// no promises here, this tries to zip the longest, so you will get fun things if you change things in the middle
pub fn combine_boop_vecs_for_world<Src: BoopFromWorld<Target> + Clone, Target: Clone>(
    conf: &BoopConf,
    t: f32,
    src: &mut Vec<Src>,
    target: &Vec<Target>,
) -> (Vec<Src>, Vec<Target>) {
    src.iter_mut()
        .zip_longest(target.iter())
        .filter_map(|pair| match pair {
            itertools::EitherOrBoth::Both(x, tar) => {
                let y = x.boop(conf, t, tar);
                Some((x.clone(), y))
            }
            itertools::EitherOrBoth::Right(tar) => {
                Some((Src::boop_init_at_time(conf, t, tar), tar.clone()))
            } // a new item uh, init a new booper
            itertools::EitherOrBoth::Left(_) => None, // oh, a dropped item
        })
        .unzip()
}

pub fn combine_boop_vecs_for_init<Src: BoopFromWorld<Target> + Clone, Target: Clone>(
    conf: &BoopConf,
    t: f32,
    target: &Vec<Target>,
) -> Vec<Src> {
    target
        .iter()
        .map(|tar| Src::boop_init_at_time(conf, t, tar))
        .collect_vec()
}

// these will get deserialized, copies are over in perform, sorry

#[derive(Debug, Copy, Clone)]
pub struct BoopODEConf {
    f: f32, // freq
    z: f32, // something
    r: f32, // reaction
}
impl BoopODEConf {
    pub fn new(f: f32, z: f32, r: f32) -> Self {
        Self { f, z, r }
    }

    fn as_consts(&self) -> (f32, f32, f32) {
        let k1 = self.z / (PI * self.f);
        let k2 = 1.0 / (2.0 * PI * self.f).powi(2);
        let k3 = self.r * self.z / (2.0 * PI * self.f);

        (k1, k2, k3)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BoopConfInner {
    ODE(BoopODEConf),
    Noop,
}

#[derive(Debug, Clone)]
pub struct BoopConf {
    pub reset: bool, // if true, change immediately
    pub curr_yaml: Option<String>,
    pub current_boop: BoopConfInner,
    pub fields: HashMap<String, BoopConfInner>,
}
impl BoopConf {
    pub fn new(
        reset: bool,
        current_boop: BoopConfInner,
        fields: HashMap<String, BoopConfInner>,
    ) -> Self {
        Self {
            reset,
            curr_yaml: None,
            current_boop,
            fields,
        }
    }

    fn check_for_inner_conf(&self, name: &str) -> Option<BoopConfInner> {
        self.fields.get(name).copied()
    }

    pub fn reset(&self) -> bool {
        self.reset
    }

    pub fn copy_with_new_current_boop(&self, key: &str) -> BoopConf {
        // first update the curr location
        let curr_yaml = if let Some(yaml) = &self.curr_yaml {
            format!("{}.{}", yaml, key)
        } else {
            key.to_owned()
        };

        // next, check if there's a new boop conf inner
        let boop_conf_inner = self
            .check_for_inner_conf(&curr_yaml)
            .unwrap_or(self.current_boop);

        // and now put it together
        BoopConf {
            reset: self.reset,
            curr_yaml: Some(curr_yaml),
            current_boop: boop_conf_inner,
            fields: self.fields.clone(),
        }
    }
}

pub trait IsBoopState {
    fn is_weird_state(&self) -> bool;

    fn init_from_conf_at_time(t: f32, target: f32) -> Self;
}

#[derive(Debug, Clone, Copy)]
pub struct BoopNoopState {
    target: f32,
}
impl IsBoopState for BoopNoopState {
    fn is_weird_state(&self) -> bool {
        false
    }

    fn init_from_conf_at_time(_t: f32, target: f32) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BoopState {
    ODE(BoopODEState),
    Noop(BoopNoopState),
}
impl BoopState {
    fn is_weird_state(&self) -> bool {
        match self {
            BoopState::ODE(x) => x.is_weird_state(),
            BoopState::Noop(x) => x.is_weird_state(),
        }
    }

    fn new(conf: &BoopConf, t: f32, target: f32) -> Self {
        match conf.current_boop {
            BoopConfInner::ODE(_o) => BoopState::ODE(BoopODEState::new(t, target)),
            BoopConfInner::Noop => BoopState::Noop(BoopNoopState { target }),
        }
    }

    fn reset(&mut self, x: f32, t: f32) {
        match self {
            BoopState::ODE(s) => s.reset(x, t),
            BoopState::Noop(s) => s.target = t,
        }
    }

    fn _boop(&mut self, conf: &BoopConf, t: f32, target: f32) -> (Option<BoopState>, f32) {
        let (maybe_new, result) = match (conf.current_boop, self) {
            (BoopConfInner::ODE(c), BoopState::ODE(o)) => (None, o._boop(&c, t, target)),
            (BoopConfInner::Noop, BoopState::Noop(x)) => (None, x.target),
            (BoopConfInner::ODE(c), _) => {
                let mut n = BoopODEState::new(t, target);
                let result = n._boop(&c, t, target);
                (Some(BoopState::ODE(n)), result)
            }
            (BoopConfInner::Noop, _) => (Some(BoopState::Noop(BoopNoopState { target })), target),
        };

        (maybe_new, result)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoopODEState {
    y: f32,      // curr val
    yd: f32,     // curr velocity
    prev_x: f32, // previous target
    prev_t: f32, // last timestamp (todo, this is global, do i need)
    weird_state: bool,
}
impl BoopODEState {
    pub fn new(time: f32, target: f32) -> Self {
        Self {
            y: target,
            yd: 0.0,
            prev_x: target,
            prev_t: time,
            weird_state: false,
        }
    }

    pub fn reset(&mut self, x: f32, t: f32) {
        self.y = x;
        self.yd = 0.0;
        self.prev_x = x;
        self.prev_t = t;
    }

    // if it went infinite and we had to reset
    pub fn is_weird_state(&self) -> bool {
        self.weird_state
    }

    // from t3ssel8r
    pub fn update(&mut self, conf: &BoopODEConf, time: f32, target: f32) -> f32 {
        let x = target;
        let t = time;

        self.weird_state = false;

        let (k1, k2, k3) = conf.as_consts();

        let t = t - self.prev_t;

        // compute xd
        let xd = (x - self.prev_x) / t;

        // update previous
        self.prev_x = x;
        self.prev_t = t;

        let k2_stable = k2.max(t.powi(2) * 0.5 + t * k1 * 0.5).max(t * k1);
        self.y += t * self.yd;
        self.yd = self.yd + t * (x + k3 * xd - self.y - k1 * self.yd) / k2_stable;

        if self.y.is_infinite() || self.y.is_nan() {
            self.reset(x, t);
            self.weird_state = true;
        }

        self.y
    }

    pub fn loc(&self) -> f32 {
        self.y
    }

    fn _boop(&mut self, conf: &BoopODEConf, t: f32, target: f32) -> f32 {
        let x = target;
        self.update(conf, t, x)
    }
}
