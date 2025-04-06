//! Since I need to do so much transforms of vec2, this
//! trait just makes it easier to use different types to
//! do that.
use glam::{vec2, vec3, Mat2, Mat3, Mat4, Vec2, Vec3};
use itertools::Itertools;
use lerpable::Lerpable;

use crate::{
    lerp,
    polyline::{IsPolyline, Polyline},
    vec_lerp, AnglePi, IsAngle,
};

pub trait TransformVec2 {
    fn transform_vec2(&self, v: Vec2) -> Vec2;

    fn transform_many_vec2<F: IsPolyline>(&self, v: &F) -> Polyline {
        v.into_iter_vec2()
            .map(|x| self.transform_vec2(x))
            .collect_vec()
            .as_polyline()
    }
}

impl TransformVec2 for Mat4 {
    fn transform_vec2(&self, v: Vec2) -> Vec2 {
        let v2 = self.transform_point3(vec3(v.x, v.y, 1.0));
        vec2(v2.x / v2.z, v2.y / v2.z)
    }
}

impl TransformVec2 for Mat3 {
    fn transform_vec2(&self, v: Vec2) -> Vec2 {
        self.transform_point2(v)
    }
}

impl FromTranslate for Mat3 {
    fn from_vec2_translate(v: Vec2) -> Self {
        Mat3::from_translation(v)
    }
}

impl TransformVec2 for Mat2 {
    fn transform_vec2(&self, v: Vec2) -> Vec2 {
        *self * v
    }
}

pub fn mat4_from_mat3_transform(m: Mat3) -> Mat4 {
    // hmm, need to keep the translation in the w axis...

    let [x_axis, y_axis, [z_x, z_y, z_z]] = m.to_cols_array_2d();
    let x_axis = Vec3::from_slice(&x_axis);
    let y_axis = Vec3::from_slice(&y_axis);
    // let z_axis = Vec3::from_slice(&z_axis);

    // i don't know about this..
    // println!("_z_z {:?}", _z_z);
    if z_z != 1.0 {
        println!("trying to turn a mat3 to mat4 with invalid values");
        println!("z_z {:?}", z_z);
        println!("m {:?}", m);
    }
    let z_axis = vec3(z_x, z_y, 0.0);

    Mat4::from_cols(
        x_axis.extend(0.0),
        y_axis.extend(0.0),
        Vec3::Z.extend(0.0),
        z_axis.extend(1.0),
    )
}

pub trait FromTranslate {
    fn from_vec2_translate(v: Vec2) -> Self;
}

impl FromTranslate for Mat4 {
    fn from_vec2_translate(v: Vec2) -> Self {
        mat4_from_mat3_transform(Mat3::from_vec2_translate(v))
    }
}

// experimental way to do a transform with a function
pub trait Vec2TransformFunction: Fn(Vec2) -> Vec2 + Send + Sync {
    fn clone_box(&self) -> Box<dyn Vec2TransformFunction>;
}

impl<T> Vec2TransformFunction for T
where
    T: 'static + Fn(Vec2) -> Vec2 + Clone + Send + Sync,
{
    fn clone_box(&self) -> Box<dyn Vec2TransformFunction> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub enum SimpleTransform2dStep {
    Translate(Vec2),
    Rotate(Vec2, AnglePi),
    Scale(Vec2),
    Skew(Vec2, Vec2),
}
impl SimpleTransform2dStep {
    pub fn translate(v: Vec2) -> Self {
        Self::Translate(v)
    }

    pub fn rotate_pi<A: IsAngle>(angle_pi: A) -> Self {
        Self::Rotate(Vec2::ZERO, angle_pi.as_angle_pi())
    }

    pub fn scale_both(v: f32) -> Self {
        Self::Scale(Vec2::ONE * v)
    }

    pub fn transform(&self) -> Mat3 {
        match self {
            Self::Translate(v) => Mat3::from_translation(*v),
            Self::Rotate(center, amount_pi) => {
                let move_to_origin = Mat3::from_translation(*center);
                let move_from_origin = Mat3::from_translation(-*center);
                let rotate = Mat3::from_angle(amount_pi.angle());
                move_from_origin * rotate * move_to_origin
            }
            Self::Scale(v) => Mat3::from_scale(*v),
            Self::Skew(v0, v1) => Mat3::from_mat2(Mat2::from_cols(*v0, *v1)),
        }
    }

    pub fn experimental_lerp(&self, other: &SimpleTransform2dStep, pct: f32) -> Self {
        match (self, other) {
            (SimpleTransform2dStep::Translate(v0), SimpleTransform2dStep::Translate(v1)) => {
                SimpleTransform2dStep::Translate(vec_lerp(v0, v1, pct))
            }
            (SimpleTransform2dStep::Rotate(v0, a0), SimpleTransform2dStep::Rotate(v1, a1)) => {
                SimpleTransform2dStep::Rotate(
                    vec_lerp(v0, v1, pct),
                    AnglePi::new(lerp(a0.angle_pi(), a1.angle_pi(), pct)),
                )
            }
            (SimpleTransform2dStep::Scale(v0), SimpleTransform2dStep::Scale(v1)) => {
                SimpleTransform2dStep::Scale(vec_lerp(v0, v1, pct))
            }
            _ => {
                if pct > 0.5 {
                    other.clone()
                } else {
                    self.clone()
                }
            }
        }
    }
}

impl Lerpable for SimpleTransform2dStep {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        self.experimental_lerp(other, pct.lerp_pct() as f32)
    }
}

#[derive(Clone, Debug)]
pub struct SimpleTransform2d(Vec<SimpleTransform2dStep>);
impl SimpleTransform2d {
    pub fn new(v: Vec<SimpleTransform2dStep>) -> Self {
        Self(v)
    }

    pub fn rotate_pi(angle_pi: f32) -> Self {
        Self(vec![SimpleTransform2dStep::rotate_pi(AnglePi::new(
            angle_pi,
        ))])
    }

    pub fn noop() -> Self {
        Self(vec![])
    }

    pub fn steps(&self) -> &Vec<SimpleTransform2dStep> {
        &self.0
    }

    pub fn add_after(&self, transform_vertex: &SimpleTransform2d) -> SimpleTransform2d {
        // just append
        let v = self
            .0
            .iter()
            .chain(transform_vertex.0.iter())
            .cloned()
            .collect();

        SimpleTransform2d(v)
    }

    pub fn ident() -> SimpleTransform2d {
        SimpleTransform2d(vec![])
    }

    pub fn translate(v: Vec2) -> Self {
        Self(vec![SimpleTransform2dStep::Translate(v)])
    }

    pub fn to_mat3(&self) -> Mat3 {
        self.0
            .iter()
            .fold(Mat3::IDENTITY, |acc, el| el.transform() * acc)
    }

    pub fn to_mat4(&self) -> Mat4 {
        mat4_from_mat3_transform(self.to_mat3())
    }
}

impl TransformVec2 for SimpleTransform2d {
    fn transform_vec2(&self, v: Vec2) -> Vec2 {
        let mut v = v;
        for step in &self.0 {
            v = step.transform().transform_vec2(v);
        }
        v
    }
}

impl Lerpable for SimpleTransform2d {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        Self::new(self.0.lerpify(&other.0, pct))
    }
}
