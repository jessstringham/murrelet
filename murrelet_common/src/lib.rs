#[allow(dead_code)]
use glam::{vec2, Vec2};
use glam::{vec3, Vec3};
use itertools::Itertools;
use lerpable::{IsLerpingMethod, Lerpable};
use num_traits::NumCast;
use std::collections::HashMap;
use std::hash::DefaultHasher;
use std::hash::{Hash, Hasher};

pub mod intersection;

mod assets;
mod color;
mod geometry;
mod idx;
mod iter;
mod metric;
mod polyline;
mod transform;

pub use assets::*;
pub use color::*;
pub use geometry::*;
pub use idx::*;
pub use iter::*;
pub use metric::*;
pub use polyline::*;
pub use transform::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

//
// TIME UTILS
//

// just a wrapper..
// since i can't use SystemTime in wasm.
// this isn't as clever with duration vs systemtime, but it gets the job done..
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MurreletTime(u128); // millis

impl MurreletTime {
    pub fn now() -> Self {
        MurreletTime(epoch_time_ms())
    }

    pub fn epoch() -> Self {
        Self(0)
    }

    pub fn from_epoch_time(t: u128) -> Self {
        Self(t)
    }

    pub fn in_x_ms(x: u128) -> Self {
        MurreletTime(epoch_time_ms() + x)
    }

    pub fn in_one_sec() -> Self {
        MurreletTime::in_x_ms(1000)
    }

    pub fn as_millis_u128(&self) -> u128 {
        self.0
    }

    pub fn as_secs(&self) -> u64 {
        (self.0 / 1000) as u64
    }

    // f32 for historical reasons, can change at some point
    pub fn as_secs_f32(&self) -> f32 {
        (self.0 as f32) / 1000.0
    }

    pub fn as_millis(&self) -> f32 {
        self.0 as f32
    }
}

impl std::ops::Sub for MurreletTime {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

// in seconds
pub fn epoch_time_ms() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        //use wasm_bindgen::JsCast;

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = Date)]
            fn now() -> f64;
        }

        now() as u128
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::SystemTime;
        use std::time::UNIX_EPOCH;
        let s = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("wat")
            .as_millis() as f64
            / 1000.0;
        (s * 1000.0) as u128
    }
}

//
// RUN ID
//

pub fn run_id() -> u64 {
    MurreletTime::now().as_secs()
}

// todo, this has something funny happening at the end where it jumps
pub fn ease(src: f64, mult: f64, offset: f64) -> f64 {
    let raw = ((src * mult + offset) % 2.0 - 1.0).abs();
    raw * raw * (3.0 - 2.0 * raw)
}

pub fn smoothstep(t: f64, edge0: f64, edge1: f64) -> f64 {
    let raw = clamp((t - edge0) / (edge1 - edge0), 0.0, 1.0);
    raw * raw * (3.0 - 2.0 * raw)
}

pub fn lerp<T>(start: T, end: T, pct: f32) -> T
where
    T: std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
    f32: std::ops::Mul<T, Output = T>,
{
    (1.0 - pct) * start + pct * end
}

pub fn lerp_vec<T>(start: &[T], end: &[T], pct: f32) -> Vec<T>
where
    T: std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T> + Copy,
    f32: std::ops::Mul<T, Output = T>,
{
    start
        .iter()
        .zip(end.iter())
        .map(|(a, b)| lerp(*a, *b, pct))
        .collect_vec()
}

// excluding start/end
pub fn lerp_x_points_between<T>(start: T, end: T, point_count: usize) -> Vec<T>
where
    T: std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T> + Copy,
    f32: std::ops::Mul<T, Output = T>,
{
    let mut result = Vec::with_capacity(point_count);
    for i in 1..point_count {
        let pct: f32 = i as f32 / (point_count - 1) as f32;
        result.push(lerp(start, end, pct));
    }
    result
}

pub fn vec_lerp(a: &Vec2, b: &Vec2, pct: f32) -> Vec2 {
    vec2(lerp(a.x, b.x, pct), lerp(a.y, b.y, pct))
}

// inclusive
pub fn clamp<T>(v: T, start: T, end: T) -> T
where
    T: PartialOrd,
{
    if v < start {
        start
    } else if v > end {
        end
    } else {
        v
    }
}

pub fn map_range<T, U>(v: T, in_min: T, in_max: T, out_min: U, out_max: U) -> U
where
    T: NumCast,
    U: NumCast,
{
    // recenter to one/zero

    let v_num: f64 = NumCast::from(v).expect("v didn't convert");
    let in_min_num: f64 = NumCast::from(in_min).expect("in_min didn't convert");
    let in_max_num: f64 = NumCast::from(in_max).expect("in_max didn't convert");
    let out_min_num: f64 = NumCast::from(out_min).expect("out_min didn't convert");
    let out_max_num: f64 = NumCast::from(out_max).expect("out_max didn't convert");

    let pct = (v_num - in_min_num) / (in_max_num - in_min_num);
    NumCast::from(out_min_num + pct * (out_max_num - out_min_num)).expect("map_range failed")
}

pub fn print_expect<T, E>(result: Result<T, E>, msg: &str) -> Option<T>
where
    E: std::fmt::Debug,
{
    match result {
        Ok(val) => Some(val),
        Err(e) => {
            println!("{}: {:?}", msg, e);
            None
        }
    }
}

pub fn cubic_bezier(start: Vec2, ctrl1: Vec2, ctrl2: Vec2, to: Vec2, t: f32) -> Vec2 {
    let a = lerp(start, ctrl1, t);
    let b = lerp(ctrl1, ctrl2, t);
    let c = lerp(ctrl2, to, t);
    let d = lerp(a, b, t);
    let e = lerp(b, c, t);
    lerp(d, e, t)
}

// creates the points from [0, 1)
pub fn smooth_interpolate(
    prev: Vec2,
    start: Vec2,
    to: Vec2,
    next: Vec2,
    length_perc: f32,
    smooth_count: usize,
) -> Vec<Vec2> {
    // make the plan
    let dist = (to - start).length();

    // average the prev to start and start and to
    let start_tangent = (0.5 * ((start - prev) + (to - start))).normalize();
    let to_tangent = -(0.5 * ((next - to) + (to - start))).normalize();

    let ctrl1 = start + length_perc * dist * start_tangent;
    let ctrl2 = to + length_perc * dist * to_tangent;

    let mut v = Vec::with_capacity(smooth_count);

    for i in 0..smooth_count {
        let t = i as f32 / smooth_count as f32;
        let p = cubic_bezier(start, ctrl1, ctrl2, to, t);
        v.push(p);
    }
    v
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    xy: Vec2,
    wh: Vec2,
}
impl Rect {
    pub fn new(xy: Vec2, wh: Vec2) -> Self {
        Self { xy, wh }
    }

    pub fn from_xy_wh(xy: Vec2, wh: Vec2) -> Self {
        Self::new(xy, wh)
    }

    pub fn from_corners(corner1: Vec2, corner2: Vec2) -> Rect {
        let xy = 0.5 * (corner1 + corner2);
        let w = (corner1.x - corner2.x).abs();
        let h = (corner1.y - corner2.y).abs();
        Rect { xy, wh: vec2(w, h) }
    }

    pub fn contains(&self, v: Vec2) -> bool {
        v.x >= self.xy.x - 0.5 * self.wh.x
            && v.x <= self.xy.x + 0.5 * self.wh.x
            && v.y >= self.xy.y - 0.5 * self.wh.y
            && v.y <= self.xy.y + 0.5 * self.wh.y
    }

    pub fn new_centered_square(rad: f32) -> Self {
        Rect {
            xy: Vec2::ZERO,
            wh: vec2(rad * 2.0, rad * 2.0),
        }
    }

    pub fn to_vec2(&self) -> Vec<Vec2> {
        vec![
            self.bottom_left(),
            self.top_left(),
            self.top_right(),
            self.bottom_right(),
        ]
    }

    pub fn to_polyline(&self) -> Polyline {
        Polyline::new(self.to_vec2())
    }

    pub fn bottom_left(&self) -> Vec2 {
        vec2(self.left(), self.bottom())
    }

    pub fn bottom_right(&self) -> Vec2 {
        vec2(self.right(), self.bottom())
    }

    pub fn top_left(&self) -> Vec2 {
        vec2(self.left(), self.top())
    }

    pub fn top_right(&self) -> Vec2 {
        vec2(self.right(), self.top())
    }

    pub fn bottom(&self) -> f32 {
        self.xy.y - 0.5 * self.wh.y
    }

    pub fn top(&self) -> f32 {
        self.xy.y + 0.5 * self.wh.y
    }

    pub fn left(&self) -> f32 {
        self.xy.x - 0.5 * self.wh.x
    }

    pub fn right(&self) -> f32 {
        self.xy.x + 0.5 * self.wh.x
    }

    pub fn w(&self) -> f32 {
        self.wh.x
    }

    pub fn h(&self) -> f32 {
        self.wh.y
    }

    pub fn wh(&self) -> Vec2 {
        self.wh
    }

    pub fn pad(&self, amount: f32) -> Self {
        Self {
            xy: self.xy + 2.0 * Vec2::ONE * amount,
            wh: self.wh,
        }
    }

    pub fn shift_y(&self, amount: f32) -> Self {
        Self {
            xy: self.xy + vec2(0.0, 1.0) * amount,
            wh: self.wh,
        }
    }

    pub fn overlap(&self, other: Rect) -> Option<Rect> {
        let has_overlap = self.right() > other.left()
            && self.left() < other.right()
            && self.top() > other.bottom()
            && self.bottom() < other.top();

        if has_overlap {
            let new_left = self.left().min(other.left());
            let new_right = self.right().min(other.right());
            let new_top = self.top().min(other.top());
            let new_bottom = self.bottom().min(other.bottom());

            let new_wh = vec2(new_right - new_left, new_top - new_bottom);

            let new_xy = vec2(new_left + new_wh.x * 0.5, new_bottom + new_wh.y * 0.5);

            Some(Rect::new(new_xy, new_wh))
        } else {
            None
        }
    }

    pub fn xy(&self) -> Vec2 {
        self.xy
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}
impl Circle {
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn new_centered(radius: f32) -> Self {
        Self {
            center: Vec2::ZERO,
            radius,
        }
    }

    pub fn diameter(&self) -> f32 {
        AnglePi::new(2.0).scale(self.radius).angle()
    }

    pub fn move_center<A: IsAngle>(&self, amount: Vec2, a_to_b_dir: &A) -> Self {
        let a: Angle = a_to_b_dir.as_angle_pi().into();

        Self {
            center: a.to_mat3().transform_point2(self.center) + amount,
            radius: self.radius,
        }
    }

    pub fn set_center(&self, center: Vec2) -> Circle {
        Circle {
            center,
            radius: self.radius,
        }
    }

    pub fn move_center_mat4(&self, transform: glam::Mat4) -> Circle {
        self.set_center(transform.transform_vec2(self.center))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LivecodeValue {
    Float(f64),
    Bool(bool),
    Int(i64),
}

impl LivecodeValue {
    #[inline]
    pub fn float(f: f32) -> Self {
        Self::Float(f as f64) // just a convenience thing
    }
}

// I use this to send data back to LiveCodeSrc's, e.g. make a MIDI controller
// LED glow when it's used in the config.
#[derive(Debug, Clone)]
pub struct LivecodeUsage {
    pub name: String,
    pub is_used: bool,
    pub value: Option<f32>,
}

impl LivecodeUsage {
    pub fn new(name: String, is_used: bool, value: Option<f32>) -> Self {
        Self {
            name,
            is_used,
            value,
        }
    }
}

pub trait IsLivecodeSrc {
    fn update(&mut self, input: &LivecodeSrcUpdateInput);
    fn to_exec_funcs(&self) -> Vec<(String, LivecodeValue)>;
    // this is a way to give usage feedback to the livecode src, e.g. tell a MIDI controller
    // we're using a parameter, or what value to set indicator lights to.
    fn feedback(&mut self, _variables: &HashMap<String, LivecodeUsage>) {
        // default don't do anything
    }
}

pub struct LivecodeSrc {
    vs: Vec<Box<dyn IsLivecodeSrc>>,
}

// what is sent from apps (like nannou)
#[derive(Default)]
pub struct MurreletAppInput {
    pub keys: Option<[bool; 26]>,
    pub window_dims: Vec2,
    pub mouse_position: Vec2,
    pub mouse_left_is_down: bool,
    pub elapsed_frames: u64,
}

impl MurreletAppInput {
    pub fn new(
        keys: [bool; 26],
        window_dims: Vec2,
        mouse_position: Vec2,
        mouse_left_is_down: bool,
        elapsed_frames: u64,
    ) -> Self {
        Self {
            keys: Some(keys),
            window_dims,
            mouse_position,
            mouse_left_is_down,
            elapsed_frames,
        }
    }

    pub fn new_no_key(
        window_dims: Vec2,
        mouse_position: Vec2,
        mouse_left_is_down: bool,
        elapsed_frames: u64,
    ) -> Self {
        Self {
            keys: None,
            window_dims,
            mouse_position,
            mouse_left_is_down,
            elapsed_frames,
        }
    }

    pub fn default_with_frames(elapsed_frames: u64) -> Self {
        let mut d = Self::default();
        d.elapsed_frames = elapsed_frames;
        d
    }

    pub fn elapsed_frames(&self) -> u64 {
        self.elapsed_frames
    }
}

// some special global things go in here
// like app (javascript input or nannou), config, (and potentially time).
// other Livecode sources will get their info
// from threads
pub struct LivecodeSrcUpdateInput<'a> {
    is_debug: bool,
    app: &'a MurreletAppInput,
    should_reset: bool,
}

impl<'a> LivecodeSrcUpdateInput<'a> {
    pub fn new(is_debug: bool, app: &'a MurreletAppInput, should_reset: bool) -> Self {
        Self {
            is_debug,
            app,
            should_reset,
        }
    }

    pub fn app(&self) -> &MurreletAppInput {
        self.app
    }

    pub fn should_reset(&self) -> bool {
        self.should_reset
    }

    pub fn is_debug(&self) -> bool {
        self.is_debug
    }
}

impl LivecodeSrc {
    pub fn new(vs: Vec<Box<dyn IsLivecodeSrc>>) -> Self {
        Self { vs }
    }

    pub fn update(&mut self, input: &LivecodeSrcUpdateInput) {
        // todo, use debug
        for v in self.vs.iter_mut() {
            v.update(input)
        }
    }

    pub fn to_world_vals(&self) -> Vec<(String, LivecodeValue)> {
        self.vs.iter().flat_map(|v| v.to_exec_funcs()).collect_vec()
    }

    pub fn feedback(&mut self, variables: &HashMap<String, LivecodeUsage>) {
        for v in self.vs.iter_mut() {
            v.feedback(variables);
        }
    }
}

const MAX_STRID_LEN: usize = 16;

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StrId([u8; MAX_STRID_LEN]);

// from chatgpt
impl StrId {
    pub fn new(s: &str) -> Self {
        let mut bytes = [0u8; MAX_STRID_LEN];
        let len = s.len().min(MAX_STRID_LEN);
        bytes[..len].copy_from_slice(&s.as_bytes()[..len]);
        StrId(bytes)
    }

    pub fn as_str(&self) -> &str {
        let len = self.0.iter().position(|&x| x == 0).unwrap_or(MAX_STRID_LEN);
        std::str::from_utf8(&self.0[..len]).unwrap()
    }

    pub fn to_seed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }

    // ah, doesn't have to be that random
    pub fn to_rn(&self) -> f32 {
        (self.to_seed() as f64 / u64::MAX as f64) as f32
    }
}

impl std::fmt::Display for StrId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Debug for StrId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format the Identifier using its string representation
        write!(f, "StrId({})", self.as_str())
    }
}

pub fn fixed_pt_f32_to_str(x: f32) -> String {
    FixedPointF32::new(x).to_str()
}

#[derive(Debug, Copy, Clone, Ord, Eq, PartialEq, PartialOrd, Hash)]
pub struct FixedPointF32 {
    pub x: i64,
}
impl FixedPointF32 {
    pub const MAX: Self = FixedPointF32 { x: i64::MAX };
    pub const MIN: Self = FixedPointF32 { x: i64::MIN };

    fn f32_to_i64(f: f32) -> i64 {
        (f * 1e4f32) as i64
    }

    fn i64_to_f32(f: i64) -> f32 {
        f as f32 / 1e4f32
    }

    pub fn to_i64(&self) -> i64 {
        self.x
    }

    pub fn round(&self, n: i64) -> Self {
        let x = self.x / n;
        Self { x }
    }

    pub fn new(x: f32) -> Self {
        Self {
            x: Self::f32_to_i64(x),
        }
    }

    fn new_from_i64(x: i64) -> Self {
        Self { x }
    }

    pub fn to_f32(&self) -> f32 {
        Self::i64_to_f32(self.x)
    }

    pub fn to_str(&self) -> String {
        self.x.to_string()
    }

    pub fn nudge(&self, x: i64) -> Self {
        Self { x: self.x + x }
    }
}

#[derive(Debug, Copy, Clone, Ord, Eq, PartialEq, PartialOrd, Hash)]
pub struct FixedPointVec2 {
    pub x: FixedPointF32,
    pub y: FixedPointF32,
}
impl FixedPointVec2 {
    pub fn round(&self, n: i64) -> FixedPointVec2 {
        FixedPointVec2::new_from_fixed_point(self.x.round(n), self.y.round(n))
    }

    pub fn new_from_i64(x: i64, y: i64) -> FixedPointVec2 {
        FixedPointVec2 {
            x: FixedPointF32::new_from_i64(x),
            y: FixedPointF32::new_from_i64(y),
        }
    }

    pub fn new(x: f32, y: f32) -> FixedPointVec2 {
        FixedPointVec2 {
            x: FixedPointF32::new(x),
            y: FixedPointF32::new(y),
        }
    }

    pub fn from_vec2(v: Vec2) -> FixedPointVec2 {
        FixedPointVec2::new(v.x, v.y)
    }

    pub fn to_vec2(&self) -> Vec2 {
        vec2(self.x.to_f32(), self.y.to_f32())
    }

    pub fn id(&self) -> StrId {
        // hopefully will be unique enough..
        let sx = self.x.to_i64().to_string();

        let sx8 = if sx.len() > 7 {
            &sx[sx.len() - 7..]
        } else {
            &sx
        };
        let sy = self.y.to_i64().to_string();
        // let sy8 = &sy[sy.len() - 8..];

        let sy8 = if sy.len() > 7 {
            &sy[sy.len() - 7..]
        } else {
            &sy
        };

        let result = format!("{:0>8}{:0>8}", sx8, sy8);

        StrId::new(&result)
    }

    fn new_from_fixed_point(x: FixedPointF32, y: FixedPointF32) -> FixedPointVec2 {
        Self { x, y }
    }

    pub fn as_tuple(&self) -> (i64, i64) {
        (self.x.to_i64(), self.y.to_i64())
    }

    pub fn nudge(&self, x: i64, y: i64) -> Self {
        Self::new_from_fixed_point(self.x.nudge(x), self.y.nudge(y))
    }
}

pub fn approx_eq_eps(x: f32, y: f32, eps: f32) -> bool {
    (x - y).abs() <= eps
}

// hrmm, wrapper types for glam and evalexpr and such

#[macro_export]
macro_rules! newtype_wrapper {
    ($wrapper:ident, $wrapped:ty) => {
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $wrapper(pub $wrapped);

        impl std::ops::Deref for $wrapper {
            type Target = $wrapped;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $wrapper {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl From<$wrapped> for $wrapper {
            fn from(value: $wrapped) -> Self {
                $wrapper(value)
            }
        }

        impl From<$wrapper> for $wrapped {
            fn from(wrapper: $wrapper) -> Self {
                wrapper.0
            }
        }
    };
}

newtype_wrapper!(MVec2, Vec2);
newtype_wrapper!(MVec3, Vec3);

impl Lerpable for MVec2 {
    fn lerpify<T: IsLerpingMethod>(&self, other: &Self, method: &T) -> Self {
        vec2(
            self.x.lerpify(&other.x, method),
            self.y.lerpify(&other.y, method),
        )
        .into()
    }
}

pub fn lerpify_vec2<T: lerpable::IsLerpingMethod>(this: &Vec2, other: &Vec2, pct: &T) -> Vec2 {
    let this: MVec2 = (*this).into();
    let other: MVec2 = (*other).into();

    this.lerpify(&other, pct).into()
}

pub fn lerpify_vec_vec2<T: lerpable::IsLerpingMethod>(
    this: &[Vec2],
    other: &[Vec2],
    pct: &T,
) -> Vec<Vec2> {
    let this_vec: Vec<MVec2> = this.iter().map(|x| (*x).into()).collect_vec();
    let other_vec: Vec<MVec2> = other.iter().map(|x| (*x).into()).collect_vec();

    let lerped = this_vec.lerpify(&other_vec, pct);

    lerped.into_iter().map(|v| v.into()).collect_vec()
}

impl Lerpable for MVec3 {
    fn lerpify<T: IsLerpingMethod>(&self, other: &Self, method: &T) -> Self {
        vec3(
            self.x.lerpify(&other.x, method),
            self.y.lerpify(&other.y, method),
            self.z.lerpify(&other.z, method),
        )
        .into()
    }
}

pub fn lerpify_vec3<T: lerpable::IsLerpingMethod>(this: &Vec3, other: &Vec3, pct: &T) -> Vec3 {
    let this: MVec3 = (*this).into();
    let other: MVec3 = (*other).into();

    this.lerpify(&other, pct).into()
}

pub fn lerpify_vec_vec3<T: lerpable::IsLerpingMethod>(
    this: &[Vec3],
    other: &[Vec3],
    pct: &T,
) -> Vec<Vec3> {
    let this_vec: Vec<MVec3> = this.iter().map(|x| (*x).into()).collect_vec();
    let other_vec: Vec<MVec3> = other.iter().map(|x| (*x).into()).collect_vec();

    let lerped = this_vec.lerpify(&other_vec, pct);

    lerped.into_iter().map(|v| v.into()).collect_vec()
}

pub fn make_gui_vec2() -> murrelet_gui::MurreletGUISchema {
    murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Vec2)
}

pub fn make_gui_vec2_coords() -> murrelet_gui::MurreletGUISchema {
    murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Coords)
}

pub fn make_gui_vec3() -> murrelet_gui::MurreletGUISchema {
    murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Vec3)
}