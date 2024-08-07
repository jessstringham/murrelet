#[allow(dead_code)]
use glam::{vec2, Vec2};
use itertools::Itertools;
use num_traits::NumCast;

pub mod intersection;

mod color;
mod geometry;
mod idx;
mod iter;
mod metric;
mod polyline;
mod transform;

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

    let mut v = Vec::new();

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

pub trait IsLivecodeSrc {
    fn update(&mut self, input: &LivecodeSrcUpdateInput);
    fn to_exec_funcs(&self) -> Vec<(String, LivecodeValue)>;
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
        &self.app
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
    fn f32_to_i64(f: f32) -> i64 {
        (f * 1e4f32) as i64
    }

    fn i64_to_f32(f: i64) -> f32 {
        f as f32 / 1e4f32
    }

    fn to_i64(&self) -> i64 {
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

    fn to_f32(&self) -> f32 {
        Self::i64_to_f32(self.x)
    }

    pub fn to_str(&self) -> String {
        self.x.to_string()
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
}
