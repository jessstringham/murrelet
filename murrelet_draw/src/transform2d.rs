#![allow(dead_code)]
use std::f32::consts::PI;

use glam::*;
use murrelet_common::{
    a_pi, mat4_from_mat3_transform, AnglePi, IsAngle, IsPolyline, Polyline, SpotOnCurve,
    TransformVec2,
};
use murrelet_livecode_derive::Livecode;

#[derive(Clone, Debug, Livecode, Default)]
pub struct Transform2d(Vec<Transform2dStep>);
impl Transform2d {
    pub fn new(actions: Vec<Transform2dStep>) -> Self {
        Self(actions)
    }

    pub fn prepend_action(&mut self, actions: &[Transform2dStep]) {
        self.0 = vec![actions.to_vec(), self.0.clone()].concat();
    }

    pub fn append_one_action(&mut self, action: Transform2dStep) {
        self.0.push(action)
    }

    pub fn append_action(&mut self, actions: &[Transform2dStep]) {
        self.0 = vec![self.0.clone(), actions.to_vec()].concat();
    }

    pub fn append_transform(&mut self, t: &Transform2d) {
        self.append_action(&t.0)
    }

    pub fn prepend_transform(&mut self, t: &Transform2d) {
        self.prepend_action(&t.0)
    }

    pub fn rotate(angle_pi: f32) -> Transform2d {
        Transform2d::new(vec![Transform2dStep::Rotate(Rotate2::new(
            Vec2::ZERO,
            a_pi(angle_pi),
        ))])
    }

    pub fn scale(scale_x: f32, scale_y: f32) -> Transform2d {
        Transform2d::new(vec![Transform2dStep::Scale(V2::new(vec2(
            scale_x, scale_y,
        )))])
    }

    pub fn translate(x: f32, y: f32) -> Transform2d {
        Self::translate_vec2(vec2(x, y))
    }

    pub fn translate_vec2(v: Vec2) -> Transform2d {
        Transform2d::new(vec![Transform2dStep::Translate(V2::new(v))])
    }

    pub fn translate_resize(v: Vec2, scale: f32) -> Transform2d {
        Transform2d::new(vec![
            Transform2dStep::Scale(V2::new(vec2(scale, scale))),
            Transform2dStep::Translate(V2::new(v)),
        ])
    }

    pub fn flip(x: f32, y: f32) -> Transform2d {
        Transform2d::new(vec![Transform2dStep::Skew(V22::new(
            vec2(0.0, x),
            vec2(y, 0.0),
        ))])
    }

    pub fn noop() -> Transform2d {
        Transform2d(Vec::new())
    }

    pub fn spot(s: &SpotOnCurve) -> Self {
        Transform2d::new(vec![
            Transform2dStep::rotate_pi(s.angle()),
            Transform2dStep::translate_vec(s.loc()),
        ])
    }

    pub fn transform_many_vec2<F: IsPolyline>(&self, vs: &F) -> Polyline {
        self.to_mat4().transform_many_vec2(vs)
    }

    pub fn transform_vec2(&self, vs: Vec2) -> Vec2 {
        self.to_mat4().transform_vec2(vs)
    }

    pub fn to_mat3(&self) -> Mat3 {
        self.0
            .iter()
            .fold(Mat3::IDENTITY, |acc, el| el.transform() * acc)
    }

    pub fn to_mat4(&self) -> Mat4 {
        mat4_from_mat3_transform(self.to_mat3())
    }

    pub fn rotate_around(angle_pi: f32, v: Vec2) -> Transform2d {
        Transform2d::new(vec![Transform2dStep::Rotate(Rotate2::new(
            v,
            a_pi(angle_pi),
        ))])
    }

    pub fn new_from_scale_rotate<A: IsAngle>(s: f32, angle_pi: A) -> Transform2d {
        Transform2d::new(vec![
            Transform2dStep::scale(s / 100.0, s / 100.0),
            Transform2dStep::rotate_pi(angle_pi),
        ])
    }

    pub fn new_translate(s: Vec2) -> Transform2d {
        Transform2d::new(vec![Transform2dStep::translate_vec(s)])
    }

    // experimental
    pub fn approx_scale(&self) -> f32 {
        let mut scale = 1.0;
        for a in &self.0 {
            match a {
                Transform2dStep::Translate(_) => {}
                Transform2dStep::Rotate(_) => {}
                Transform2dStep::Scale(s) => scale *= s.v.x.max(s.v.y),
                Transform2dStep::Skew(_) => todo!(),
            }
        }
        scale
    }

    pub fn approx_rotate(&self) -> AnglePi {
        let mut rotate = AnglePi::new(0.0);
        for a in &self.0 {
            match a {
                Transform2dStep::Translate(_) => {}
                Transform2dStep::Rotate(s) => rotate = rotate + s.angle_pi(),
                Transform2dStep::Scale(_) => {}
                Transform2dStep::Skew(_) => todo!(),
            }
        }
        rotate
    }
}

impl Default for ControlTransform2d {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Default for ControlLazyTransform2d {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Clone, Debug, Livecode, Default)]
pub struct V2 {
    v: Vec2,
}

impl V2 {
    pub fn new(v: Vec2) -> Self {
        Self { v }
    }
}

#[derive(Clone, Debug, Livecode, Default)]
pub struct V22 {
    v0: Vec2,
    v1: Vec2,
}

impl V22 {
    pub fn new(v0: Vec2, v1: Vec2) -> Self {
        Self { v0, v1 }
    }
}

#[derive(Clone, Debug, Livecode, Default)]
pub struct Rotate2 {
    #[livecode(serde_default = "zeros")]
    center: Vec2,
    angle_pi: f32,
}

impl Rotate2 {
    pub fn new<A: IsAngle>(center: Vec2, angle_pi: A) -> Self {
        Self {
            center,
            angle_pi: angle_pi.angle_pi(),
        }
    }

    fn angle_pi(&self) -> AnglePi {
        AnglePi::new(self.angle_pi)
    }
}

#[derive(Clone, Debug, Livecode)]
pub enum Transform2dStep {
    Translate(V2),
    Rotate(Rotate2),
    Scale(V2),
    Skew(V22),
}
impl Default for Transform2dStep {
    fn default() -> Self {
        Transform2dStep::Translate(V2::new(Vec2::ZERO))
    }
}

impl Transform2dStep {
    pub fn rotate_pi<A: IsAngle>(angle_pi: A) -> Self {
        Self::Rotate(Rotate2::new(Vec2::ZERO, angle_pi))
    }

    pub fn rotate_pi_around(angle_pi: f32, center: Vec2) -> Self {
        Self::Rotate(Rotate2::new(center, a_pi(angle_pi)))
    }

    pub fn translate(translate_x: f32, translate_y: f32) -> Self {
        Self::translate_vec(vec2(translate_x, translate_y))
    }

    pub fn translate_vec(translate: Vec2) -> Self {
        Self::Translate(V2::new(translate))
    }

    pub fn scale(scale_x: f32, scale_y: f32) -> Self {
        Self::Scale(V2::new(vec2(scale_x, scale_y)))
    }

    fn transform(&self) -> Mat3 {
        match self {
            Transform2dStep::Translate(t) => Mat3::from_translation(t.v),
            Transform2dStep::Rotate(t) => {
                let move_to_origin = Mat3::from_translation(t.center);
                let move_from_origin = Mat3::from_translation(-t.center);
                let rotate = Mat3::from_angle(t.angle_pi * PI);
                move_from_origin * rotate * move_to_origin
            }
            Transform2dStep::Scale(t) => Mat3::from_scale(t.v),
            Transform2dStep::Skew(t) => Mat3::from_mat2(Mat2::from_cols(t.v0, t.v1)),
        }
    }
}
