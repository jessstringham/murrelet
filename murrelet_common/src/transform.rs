//! Since I need to do so much transforms of vec2, this
//! trait just makes it easier to use different types to
//! do that.
use glam::{vec2, vec3, Mat2, Mat3, Mat4, Vec2, Vec3};
use itertools::Itertools;

use crate::polyline::{IsPolyline, Polyline};

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
