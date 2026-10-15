use glam::*;
use itertools::Itertools;
use lerpable::Lerpable;
use murrelet_common::*;
use murrelet_livecode_derive::Livecode;

impl Default for ControlTransform3d {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Default for ControlLazyTransform3d {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Clone, Debug, Livecode, Lerpable, Default)]
pub struct Transform3d(Vec<Transform3dStep>);
impl Transform3d {
    pub fn noop() -> Self {
        Self(vec![])
    }

    pub fn new(transform3d_steps: Vec<Transform3dStep>) -> Self {
        Self(transform3d_steps)
    }

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

    pub fn rotation_transform(&self) -> Self {
        let steps = self
            .0
            .iter()
            .filter(|x| matches!(x, Transform3dStep::Rotate(_)))
            .cloned()
            .collect_vec();
        Self(steps)
    }

    pub fn rotation_quat(&self) -> Quat {
        let mat = self.rotation_transform().to_mat4();
        Quat::from_mat4(&mat)
    }

    pub fn to_mat4(&self) -> Mat4 {
        self.0
            .iter()
            .map(|x| x.transform())
            .fold(Mat4::IDENTITY, |acc, el| el * acc)
    }

    pub fn translate(v: Vec3) -> Transform3d {
        Self(vec![Transform3dStep::translate(v)])
    }

    pub fn rotate<Axis: IsAxis3d, A: IsAngle>(axis: &Axis, angle: A) -> Transform3d {
        Transform3d::new(vec![Transform3dStep::rotate(axis, angle)])
    }

    pub fn rotate_around<Axis: IsAxis3d, A: IsAngle>(
        axis: &Axis,
        angle: A,
        origin: Vec3,
    ) -> Transform3d {
        Transform3d::new(vec![Transform3dStep::rotate_around(axis, angle, origin)])
    }

    pub fn invert(&self) -> Self {
        let mut v = vec![];
        for step in self.0.iter().rev() {
            v.push(step.invert())
        }

        Self(v)
    }

    pub fn concat(&self, t: Transform3d) -> Transform3d {
        let mut transforms = self.0.clone();
        transforms.extend(t.0);
        Transform3d(transforms)
    }

    pub fn prepend(self, t: Transform3d) -> Transform3d {
        let mut transforms = vec![];
        transforms.extend(t.0);
        transforms.extend(self.0);
        Transform3d(transforms)
    }

    pub fn scale_transform(&self) -> Transform3d {
        let steps = self
            .0
            .iter()
            .filter(|x| matches!(x, Transform3dStep::Scale(_)))
            .cloned()
            .collect_vec();
        Self(steps)
    }

    pub fn scale_scalar(&self) -> f32 {
        self.scale_transform().transform(&Vec3::ONE).x // hax?
    }

    /// to do, maybe this shoudl just be it's own
    pub fn look_at(eye: Vec3, target: Vec3) -> Transform3d {
        Self::look_at_up(eye, target, Vec3::Y)
    }

    pub fn look_at_up(eye: Vec3, target: Vec3, up: Vec3) -> Transform3d {
        let dir = (target - eye).normalize_or(-Vec3::Z);
        // if we're looking straight along `up`, look_at_rh goes NaN, so pick another up
        let up = if dir.cross(up).length_squared() < 1e-6 {
            if up.dot(Vec3::Y).abs() > 0.9 {
                Vec3::Z
            } else {
                Vec3::Y
            }
        } else {
            up
        };

        let (_scale, rot, translate) =
            Mat4::look_at_rh(eye, eye + dir, up).to_scale_rotation_translation();
        let (axis, angle) = rot.to_axis_angle();

        Transform3d::new(vec![
            Transform3dStep::rotate(&axis, Angle::new(angle)),
            Transform3dStep::translate(translate),
        ])
    }
}

pub trait IsAxis3d {
    fn to_axis3d(&self) -> Axis3d;
}

impl IsAxis3d for Vec3 {
    fn to_axis3d(&self) -> Axis3d {
        Axis3d::new(*self)
    }
}

impl IsAxis3d for Axis3d {
    fn to_axis3d(&self) -> Axis3d {
        self.clone()
    }
}

pub trait CanBeTransformed3d {
    type ProjectedVersion;

    fn apply_transform3d(&self, t: &Transform3d) -> Self;

    fn project(&self) -> Self::ProjectedVersion;
}

impl CanBeTransformed3d for Vec3 {
    type ProjectedVersion = Vec2;

    fn apply_transform3d(&self, t: &Transform3d) -> Self {
        let v4 = vec4(self.x, self.y, self.z, 1.0);

        let v3 =
            t.0.iter()
                .map(|x| x.transform())
                .fold(v4, |acc, el| el * acc);

        vec3(v3.x / v3.w, v3.y / v3.w, v3.z / v3.w)
    }

    fn project(&self) -> Vec2 {
        self.xy()
    }
}

impl CanBeTransformed3d for Vec<Vec3> {
    fn apply_transform3d(&self, t: &Transform3d) -> Self {
        self.iter().map(|x| x.apply_transform3d(t)).collect_vec()
    }

    fn project(&self) -> Vec<Vec2> {
        self.iter().map(|x| x.project()).collect_vec()
    }

    type ProjectedVersion = Vec<Vec2>;
}

#[derive(Clone, Debug, Livecode, Lerpable)]
#[livecode(enum_tag = "untagged")]
pub enum Axis3d {
    V(Vec3),
    Name(MurreletString), // todo this doens't have copy...
}
impl Default for Axis3d {
    fn default() -> Self {
        Self::V(Vec3::Y)
    }
}

impl Axis3d {
    fn new(v: Vec3) -> Self {
        Self::V(v)
    }

    pub fn axis(&self) -> Vec3 {
        match self {
            Axis3d::V(vec3) => *vec3,
            Axis3d::Name(string) => match string.to_string().as_str() {
                "x" => Vec3::X,
                "y" => Vec3::Y,
                "z" => Vec3::Z,
                "-x" => -Vec3::X,
                "-y" => -Vec3::Y,
                "-z" => -Vec3::Z,
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
    pub fn new(origin: Vec3, axis: Axis3d, angle_pi: AnglePi) -> Self {
        Self {
            origin,
            axis,
            angle_pi,
        }
    }

    fn axis(&self) -> Vec3 {
        self.axis.axis().normalize_or(Vec3::Z)
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
                let move_to_origin = Mat4::from_translation(-t.origin);
                let move_from_origin = Mat4::from_translation(t.origin);
                let rotate = Mat4::from_axis_angle(t.axis(), t.angle_pi.angle());
                move_from_origin * rotate * move_to_origin
            }
            Transform3dStep::Scale(t) => Mat4::from_scale(t.v),
        }
    }

    pub fn translate(v: Vec3) -> Transform3dStep {
        Self::Translate(V3 { v })
    }

    pub fn rotate<Axis: IsAxis3d, A: IsAngle>(axis: &Axis, angle_pi: A) -> Transform3dStep {
        Transform3dStep::Rotate(Rotate3::new(
            Vec3::ZERO,
            axis.to_axis3d(),
            angle_pi.as_angle_pi(),
        ))
    }

    pub fn rotate_around<Axis: IsAxis3d, A: IsAngle>(
        axis: &Axis,
        angle_pi: A,
        origin: Vec3,
    ) -> Transform3dStep {
        Transform3dStep::Rotate(Rotate3::new(
            origin,
            axis.to_axis3d(),
            angle_pi.as_angle_pi(),
        ))
    }

    fn invert(&self) -> Transform3dStep {
        match self {
            Transform3dStep::Translate(v3) => Transform3dStep::translate(-v3.v),
            Transform3dStep::Rotate(r3) => {
                Transform3dStep::rotate_around(&r3.axis, r3.angle_pi.scale(-1.0), r3.origin)
            }
            Transform3dStep::Scale(v3) => Transform3dStep::scale(1.0 / v3.v),
        }
    }

    fn scale(v: Vec3) -> Transform3dStep {
        Transform3dStep::Scale(V3 { v })
    }
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
