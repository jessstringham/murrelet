#![allow(dead_code)]
use glam::*;
use itertools::Itertools;
use lerpable::Lerpable;
use murrelet_common::{AnglePi, IsAngle, MurreletString, SpotOnCurve};
use murrelet_livecode_derive::Livecode;

#[derive(Debug, Clone, Copy)]
pub struct SpotOn3d {
    loc: Vec3,
    rot: Quat,
}
impl SpotOn3d {
    pub fn new(loc: Vec3, rot: Quat) -> Self {
        Self { loc, rot }
    }

    pub fn radial(&self) -> Vec3 {
        self.rot * Vec3::X
    }
    pub fn tangent(&self) -> Vec3 {
        self.rot * Vec3::Y
    }
    pub fn normal(&self) -> Vec3 {
        self.rot * Vec3::Z
    }

    pub fn dir<A: IsAngle, B: IsAngle>(&self, around_stroke: A, splay: B) -> Vec3 {
        let (s, c) = around_stroke.angle().sin_cos();
        let (ss, sc) = splay.angle().sin_cos();
        self.rot * vec3(sc * c, ss, sc * s)
    }

    pub fn from2d(spot: SpotOnCurve) -> SpotOn3d {
        SpotOn3d {
            loc: spot.loc.extend(0.0),
            rot: Quat::from_rotation_z(spot.angle()._angle()),
        }
    }

    pub fn from_dir(loc: Vec3, tangent: Vec3) -> Self {
        let dir = tangent.normalize();
        let rot = Quat::from_rotation_arc(Vec3::Y, dir);
        SpotOn3d { loc, rot }
    }

    pub fn with_strength(&self, strength: f32) -> SpotOn3dStrength {
        SpotOn3dStrength {
            spot: self.clone(),
            strength,
        }
    }

    pub fn loc(&self) -> Vec3 {
        self.loc
    }

    pub fn change_dir(&self, dir: Vec3) -> Self {
        SpotOn3d::from_dir(self.loc(), dir)
    }
}

#[derive(Debug, Clone)]
pub struct SpotOn3dStrength {
    spot: SpotOn3d,
    strength: f32,
}
impl SpotOn3dStrength {
    pub fn strength(&self) -> f32 {
        self.strength
    }

    pub fn spot(&self) -> SpotOn3d {
        self.spot
    }

    pub fn loc(&self) -> Vec3 {
        self.spot.loc
    }
}

pub trait ToVec3 {
    fn to_vec3(&self) -> Vec3;
}
impl ToVec3 for SpotOn3dStrength {
    fn to_vec3(&self) -> Vec3 {
        self.spot.to_vec3()
    }
}
impl ToVec3 for SpotOn3d {
    fn to_vec3(&self) -> Vec3 {
        self.loc
    }
}

impl ToVec3 for Vec3 {
    fn to_vec3(&self) -> Vec3 {
        *self
    }
}

#[derive(Clone, Debug, Livecode, Lerpable)]
pub struct Transform3d(Vec<Transform3dStep>);
impl Transform3d {
    pub fn transform<V: CanBeTransformed3d>(&self, v: &V) -> V {
        v.apply_transform3d(self)
    }

    pub fn transform_many<V: CanBeTransformed3d>(&self, v: &[V]) -> Vec<V> {
        v.iter().map(|x| self.transform(x)).collect_vec()
    }

    pub fn transform_around_origin<V: CanBeTransformed3d>(&self, v: &V, origin: Vec3) -> V {
        v.apply_transform3d(&Transform3d::translate(-origin))
            .apply_transform3d(self)
            .apply_transform3d(&Transform3d::translate(origin))
    }

    fn rotation_transform(&self) -> Self {
        let steps = self
            .0
            .iter()
            .filter(|x| matches!(x, Transform3dStep::Rotate(_)))
            .cloned()
            .collect_vec();
        Self(steps)
    }

    fn rotation_quat(&self) -> Quat {
        let mat = self.rotation_transform().to_mat4();
        Quat::from_mat4(&mat)
    }

    fn to_mat4(&self) -> Mat4 {
        self.0
            .iter()
            .map(|x| x.transform())
            .fold(Mat4::IDENTITY, |acc, el| el * acc)
    }

    fn translate(v: Vec3) -> Transform3d {
        Self(vec![Transform3dStep::translate(v)])
    }
}

pub trait CanBeTransformed3d {
    fn apply_transform3d(&self, t: &Transform3d) -> Self;
}

impl CanBeTransformed3d for Vec3 {
    fn apply_transform3d(&self, t: &Transform3d) -> Self {
        let v4 = vec4(self.x, self.y, self.z, 1.0);

        let v3 =
            t.0.iter()
                .map(|x| x.transform())
                .fold(v4, |acc, el| el * acc);

        vec3(v3.x / v3.w, v3.y / v3.w, v3.z / v3.w)
    }
}

impl CanBeTransformed3d for SpotOn3d {
    fn apply_transform3d(&self, t: &Transform3d) -> Self {
        let loc = self.loc.apply_transform3d(t);
        // filter for the rotation ones
        let rotations = t.rotation_transform().rotation_quat();
        let rot = rotations * self.rot;

        Self { loc, rot }
    }
}

#[derive(Clone, Debug, Livecode, Lerpable)]
#[livecode(enum_tag = "untagged")]
pub enum Axis3d {
    V(Vec3),
    Name(MurreletString),
}
impl Axis3d {
    fn axis(&self) -> Vec3 {
        match self {
            Axis3d::V(vec3) => *vec3,
            Axis3d::Name(string) => match string.to_string().as_str() {
                "x" => Vec3::X,
                "y" => Vec3::Y,
                "z" => Vec3::Z,
                "xaxis" => Vec3::X,
                "yaxis" => Vec3::Y,
                "zaxis" => Vec3::Z,
                _ => {
                    println!("unrecognized axis name {:?}", string.to_string());
                    Vec3::Y
                }
            },
        }
    }
}

#[derive(Clone, Debug, Livecode, Lerpable)]
pub struct Rotate3 {
    #[livecode(serde_default = "0")]
    origin: Vec3,
    axis: Axis3d,
    angle_pi: AnglePi,
}
impl Rotate3 {
    fn axis(&self) -> Vec3 {
        self.axis.axis()
    }
}

#[derive(Clone, Debug, Livecode, Lerpable)]
pub struct V3 {
    v: Vec3,
}

#[derive(Clone, Debug, Livecode, Lerpable)]
pub enum Transform3dStep {
    Translate(V3),
    Rotate(Rotate3),
    Scale(V3),
}
impl Transform3dStep {
    fn transform(&self) -> Mat4 {
        match self {
            Transform3dStep::Translate(t) => Mat4::from_translation(t.v),
            Transform3dStep::Rotate(t) => {
                let move_to_origin = Mat4::from_translation(t.origin);
                let move_from_origin = Mat4::from_translation(-t.origin);
                let rotate = Mat4::from_axis_angle(t.axis(), t.angle_pi.angle());
                move_from_origin * rotate * move_to_origin
            }
            Transform3dStep::Scale(t) => Mat4::from_scale(t.v),
        }
    }

    fn translate(v: Vec3) -> Transform3dStep {
        Self::Translate(V3 { v })
    }
}

fn transform_vec2(m: Mat4, v: Vec2) -> Vec2 {
    let a = m.mul_vec4(vec4(v.x, v.y, 0.0, 1.0));
    vec2(a.x, a.y)
}

fn transform_vec3(m: Mat4, v: Vec3) -> Vec3 {
    let a = m.mul_vec4(vec4(v.x, v.y, v.z, 1.0));
    let r = m * a;

    vec3(r.x / r.w, r.y / r.w, r.z / r.w)
}

pub trait CanTransform3d {
    fn transform_mat4(&self) -> Mat4;

    fn transform_vec3(&self, v: Vec3) -> Vec3 {
        let v4 = vec4(v.x, v.y, v.z, 1.0);

        let v3 = self.transform_mat4().mul_vec4(v4);

        vec3(v3.x / v3.w, v3.y / v3.w, v3.z / v3.w)
    }
}

pub fn transform_around_origin(v: Vec3, transform: Mat4) -> Vec3 {
    let v4 = vec4(v.x, v.y, v.z, 1.0);

    let transformed = transform.mul_vec4(v4);

    vec3(
        transformed.x / transformed.w,
        transformed.y / transformed.w,
        transformed.z / transformed.w,
    )
}
