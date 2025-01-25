

pub const SUFFIX: &str = r#"
    return FragmentOutput(result);
}
"#;

pub const BINDING_2TEX: &str = r#"
struct FragmentOutput {
    @location(0) f_color: vec4<f32>,
};

struct Uniforms {
    dims: vec4<f32>,
    more_info: vec4<f32>,
    more_info_other: vec4<f32>,
};

@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var tex2: texture_2d<f32>;
@group(0) @binding(2)
var tex_sampler: sampler;

@group(0) @binding(3)
var<uniform> uniforms: Uniforms;
"#;

pub const BINDING_1TEX: &str = r#"
struct FragmentOutput {
    @location(0) f_color: vec4<f32>,
};

struct Uniforms {
    dims: vec4<f32>,
    more_info: vec4<f32>,
    more_info_other: vec4<f32>,
};

@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@group(0) @binding(2)
var<uniform> uniforms: Uniforms;
"#;



pub const BINDING_3D: &str = r#"
struct FragmentOutput {
    @location(0) f_color: vec4<f32>,
};

struct Uniforms {
    dims: vec4<f32>,
    more_info: vec4<f32>,
    more_info_other: vec4<f32>,
};

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;
@group(0) @binding(4) var shadow_map: texture_depth_2d;
@group(0) @binding(5) var shadow_sampler: sampler_comparison;

"#;


pub const INCLUDES: &str = r#"


// stealing from the internet
fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    let r = vec3<f32>(
        clamp(p.x - K.x, 0.0, 1.0),
        clamp(p.y - K.x, 0.0, 1.0),
        clamp(p.z - K.x, 0.0, 1.0));
    return c.z * mix(K.xxx, r, c.y);
}

// https://bottosson.github.io/posts/oklab/
fn oklab_to_linear_srgb(oklab: vec3<f32>) -> vec3<f32> {
    let l_ = oklab.r + 0.3963377774 * oklab.g + 0.2158037573 * oklab.b;
    let m_ = oklab.r - 0.1055613458 * oklab.g - 0.0638541728 * oklab.b;
    let s_ = oklab.r - 0.0894841775 * oklab.g - 1.2914855480 * oklab.b;

    let l = l_*l_*l_;
    let m = m_*m_*m_;
    let s = s_*s_*s_;

    return vec3<f32>(
       4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
      -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
      -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    );
}

fn linear_srgb_to_oklab(c: vec3<f32>) -> vec3<f32>
{
    let l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
  	let m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
	  let s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;

    let l_ = pow(l, 1.0/3.0);
    let m_ = pow(m, 1.0/3.0);
    let s_ = pow(s, 1.0/3.0);

    return vec3<f32>(
        0.2104542553*l_ + 0.7936177850*m_ - 0.0040720468*s_,
        1.9779984951*l_ - 2.4285922050*m_ + 0.4505937099*s_,
        0.0259040371*l_ + 0.7827717662*m_ - 0.8086757660*s_,
    );
}


fn luma(rgb: vec3<f32>) -> f32 {
  let s: vec3<f32> = pow(rgb, vec3<f32>(2.2));
  let l: f32 = 0.2126 * s.r + 0.7152 * s.g + 0.0722 * s.b;
  return pow(l, 1.0 / 2.2);
}

fn rand(n: f32) -> f32 { return fract(sin(n) * 43758.5453123); }
fn noise(p: f32) -> f32 {
  let fl = floor(p);
  let fc = fract(p);
  return mix(rand(fl), rand(fl + 1.), fc);
}

fn rand2(n: vec2<f32>) -> f32 {
  return fract(sin(dot(n, vec2<f32>(12.9898, 4.1414))) * 43758.5453);
}

// i don't know where this went
fn smoothStep(edge0: vec2<f32>, edge1: vec2<f32>, x: vec2<f32>) -> vec2<f32> {
    let t: vec2<f32> = clamp((x - edge0) / (edge1 - edge0), vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    return t * t * (vec2<f32>(3.0, 3.0) - 2.0 * t);
}


fn noise2(n: vec2<f32>) -> f32 {
  let d = vec2<f32>(0., 1.);
  let b = floor(n);
  let f = smoothStep(vec2<f32>(0.), vec2<f32>(1.), fract(n));
  return mix(mix(rand2(b), rand2(b + d.yx), f.x), mix(rand2(b + d.xy), rand2(b + d.yy), f.x), f.y);
}

fn pixel_noise2(tex_coords: vec2<f32>) -> f32 {
  let n = fract(tex_coords * uniforms.dims.y);
  let d = vec2<f32>(0., 1.);
  let b = floor(n);
  let f = smoothStep(vec2<f32>(0.), vec2<f32>(1.), fract(n));
  return mix(mix(rand2(b), rand2(b + d.yx), f.x), mix(rand2(b + d.xy), rand2(b + d.yy), f.x), f.y);
}

fn mod3(what_to_mod: vec3<f32>, what: vec3<f32>) -> vec3<f32> {
    return what_to_mod - floor(what_to_mod * 1.0 / what) * what;
}

fn step_3(input: vec3<f32>, compare: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(step(input.r, compare.r), step(input.g, compare.g), step(input.b, compare.b));
}

fn mod289(x: vec2<f32>) -> vec2<f32> {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn mod289_3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn permute3(x: vec3<f32>) -> vec3<f32> {
    return mod289_3(((x * 34.) + 1.) * x);
}

fn srandom(pos: vec3<f32>) -> f32 {
  return -1. + 2. * fract(sin(dot(pos.xyz, vec3<f32>(70.9898, 78.233, 32.4355))) * 43758.5453123);
}

// 1 if non-zero, 0 otherwise (where zero is 0.0001)
fn is_almost_nonzero(v: f32) -> f32 {
  return step(0.0001, v);
}

fn is_almost_zero(v: f32) -> f32 {
  return 1.0 - is_almost_nonzero(v);
}


fn fbm(i: vec2<f32>) -> f32 {
  var p = i;

  let m2: mat2x2<f32> = mat2x2<f32>(vec2<f32>(0.8, 0.6), vec2<f32>(-0.6, 0.8));
  var f: f32 = 0.;
  f = f + 0.5000 * noise2(p);
  p = m2 * p * 2.02;
  f = f + 0.2500 * noise2(p);
  p = m2 * p * 2.03;
  f = f + 0.1250 * noise2(p);
  p = m2 * p * 2.01;
  f = f + 0.0625 * noise2(p);
  return f / 0.9375;
}


fn clamp01(p: f32) -> f32 {
  return clamp(p, 0.0, 1.0);
}

fn clamp3(p: vec3<f32>, min: f32, max: f32) -> vec3<f32> {
  return vec3<f32>(
      clamp(p.x, 0.0, 1.0),
      clamp(p.y, 0.0, 1.0),
      clamp(p.z, 0.0, 1.0)
  );
}

fn clamp4(p: vec4<f32>, min: f32, max: f32) -> vec4<f32> {
  return vec4<f32>(
      clamp(p.x, 0.0, 1.0),
      clamp(p.y, 0.0, 1.0),
      clamp(p.z, 0.0, 1.0),
      clamp(p.a, 0.0, 1.0)
  );
}

fn color_if_for_neg_color(p: vec4<f32>) -> vec4<f32> {
  return vec4<f32>(step(0.0, p.r), step(0.0, p.g), step(0.0, p.b), 1.0);
}

fn color_if_for_over_1_color(p: vec4<f32>) -> vec4<f32> {
  return vec4<f32>(step(p.r, 1.0), step(p.g, 1.0), step(p.b, 1.0), 1.0);
}

fn red_if_alpha_over_green_if_under(p: vec4<f32>) -> vec4<f32> {
  return vec4<f32>(step(1.0, p.a), step(0.0, p.g), 0.0, 1.0);
}

fn mod289_4(x: vec4<f32>) -> vec4<f32> { return x - floor(x * (1. / 289.)) * 289.; }
fn perm4(x: vec4<f32>) -> vec4<f32> { return mod289_4(((x * 34.) + 1.) * x); }

fn noise3(p: vec3<f32>) -> f32 {
    let a = floor(p);
    var d: vec3<f32> = p - a;
    d = d * d * (3. - 2. * d);

    let b = a.xxyy + vec4<f32>(0., 1., 0., 1.);
    let k1 = perm4(b.xyxy);
    let k2 = perm4(k1.xyxy + b.zzww);

    let c = k2 + a.zzzz;
    let k3 = perm4(c);
    let k4 = perm4(c + 1.);

    let o1 = fract(k3 * (1. / 41.));
    let o2 = fract(k4 * (1. / 41.));

    let o3 = o2 * d.z + o1 * (1. - d.z);
    let o4 = o3.yw * d.x + o3.xz * (1. - d.x);

    return o4.y * d.y + o4.x * (1. - d.y);
}

// i don't know if this is right
fn toroid_noise(r1: f32, r2: f32, xy: vec2<f32>) -> f32 {
  let x = fract(xy.x);
  let y = fract(xy.y);

  let angle_x = x * 2.0 * 3.1415926535;
  let angle_y = y * 2.0 * 3.1415926535;

  // convert the coordinates to the location in the 3d torus
  let torus_coords: vec3<f32> = vec3<f32>(
      (r1 + r2 * cos(angle_y)) * cos(angle_x),
      (r1 + r2 * cos(angle_y)) * sin(angle_x),
      r2 * sin(angle_y),
  );

  let rn = noise3(torus_coords);
  return rn;
}
"#;

pub const VERTEX_SHADER: &str = "
struct VertexOutput {
  @location(0) tex_coords: vec2<f32>,
  @location(1) world_pos: vec4<f32>,
  @location(2) normal: vec3<f32>,
  @location(3) light_space_pos: vec4<f32>,
  @location(4) world_pos: vec3<f32>,
  @builtin(position) out_pos: vec4<f32>,
};

@vertex
fn main(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) face_loc: vec2<f32>) -> VertexOutput {
  let tex_coords: vec2<f32> = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
  let out_pos: vec4<f32> = vec4<f32>(pos.xy, 0.0, 1.0);
  return VertexOutput(
    tex_coords,
    vec4<f32>(0.0), // shad_info
    vec3<f32>(0.0), //normal
    vec4<f32>(0.0), //light space pos
    vec3<f32>(0.0), //world_pos,
    out_pos);
}";



pub const VERTEX_SHADER_3D: &str = "
struct Uniforms {
  view_proj: mat4x4<f32>,
  light_proj: mat4x4<f32>,
};
@group(0) @binding(3) var<uniform> uniforms: Uniforms;

struct VertexOutput {
  @location(0) tex_coords: vec2<f32>,
  @location(1) shad_info: vec4<f32>,
  @location(2) normal: vec3<f32>,
  @location(3) light_space_pos: vec4<f32>,
  @location(4) world_pos: vec3<f32>,
  @builtin(position) out_pos: vec4<f32>,
};

@vertex
fn main(@location(0) pos: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) face_loc: vec2<f32>) -> VertexOutput {
  let world_pos: vec4<f32> = vec4<f32>(pos, 1.0);
  let clip_pos: vec4<f32> = uniforms.view_proj * world_pos;
  let light_space_pos = uniforms.light_proj * vec4<f32>(pos, 1.0);

  let shad_info: vec4<f32> = vec4<f32>(face_loc, clip_pos.za);

  let tex_coords: vec2<f32> = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));

  return VertexOutput(tex_coords, shad_info, normal, light_space_pos, pos, clip_pos);

}";

pub const PREFIX: &str = r#"
@fragment
fn main(@location(0) tex_coords: vec2<f32>, @location(1) shad_info: vec4<f32>, @location(2) normal: vec3<f32>, @location(3) light_space_pos: vec4<f32>, @location(4) world_pos: vec3<f32>) -> FragmentOutput {
"#;