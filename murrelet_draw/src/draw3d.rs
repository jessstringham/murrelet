use glam::{Quat, Vec3};
use itertools::Itertools;
use murrelet_common::*;

use crate::transform3d::{CanBeTransformed3d, Transform3d};

pub fn eval_3d_bezier(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> SpotOn3d {
    let u = 1.0 - t;

    let t2 = t * t;
    let u2 = u * u;

    let loc = p0 * (u2 * u) + p1 * (3.0 * u2 * t) + p2 * (3.0 * u * t2) + p3 * (t2 * t);

    let tangent = (p1 - p0) * (3.0 * u2) + (p2 - p1) * (6.0 * u * t) + (p3 - p2) * (3.0 * t2);

    SpotOn3d::from_dir(loc, tangent)
}

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

    fn aim<A: IsAngle, B: IsAngle>(around_stroke: A, splay: B) -> Quat {
        Quat::from_rotation_y(-around_stroke.angle())
            * Quat::from_rotation_z(splay.angle() - std::f32::consts::FRAC_PI_2)
    }

    pub fn dir<A: IsAngle, B: IsAngle>(&self, around_stroke: A, splay: B) -> Vec3 {
        self.rot * Self::aim(around_stroke, splay) * Vec3::Y
    }

    pub fn dir_spot<A: IsAngle, B: IsAngle>(&self, around_stroke: A, splay: B) -> Self {
        Self {
            loc: self.loc,
            rot: (self.rot * Self::aim(around_stroke, splay)).normalize(),
        }
    }

    pub fn from2d(spot: SpotOnCurve) -> SpotOn3d {
        SpotOn3d {
            loc: spot.loc.extend(0.0),
            rot: Quat::from_rotation_z(spot.angle()._angle() - std::f32::consts::FRAC_PI_2),
        }
    }

    pub fn from_dir(loc: Vec3, tangent: Vec3) -> Self {
        let rot = if let Some(dir) = tangent.try_normalize() {
            Quat::from_rotation_arc(Vec3::Y, dir)
        } else {
            // todo, print?
            Quat::IDENTITY
        };
        SpotOn3d { loc, rot }
    }

    pub fn loc(&self) -> Vec3 {
        self.loc
    }

    pub fn change_dir(&self, dir: Vec3) -> Self {
        let Some(new_dir) = dir.try_normalize() else {
            return *self;
        };
        let tangent = self.tangent();
        // if we're in the very specal case of flipping, then just use the prev
        let carry = if tangent.dot(new_dir) < -0.999_999 {
            Quat::from_axis_angle(self.normal(), std::f32::consts::PI)
        } else {
            Quat::from_rotation_arc(tangent, new_dir)
        };
        SpotOn3d {
            loc: self.loc,
            rot: (carry * self.rot).normalize(),
        }
    }

    pub fn travel(&self, dist: f32) -> SpotOn3d {
        SpotOn3d::new(self.loc + self.tangent() * dist, self.rot)
    }

    pub fn to_transform3d(&self) -> Transform3d {
        let (axis, radians) = self.rot.to_axis_angle();

        Transform3d::rotate(&axis, Angle::new(radians)).concat(Transform3d::translate(self.loc))
    }

    pub fn at_loc(&self, loc: Vec3) -> Self {
        Self { loc, rot: self.rot }
    }

    pub fn rot(&self) -> Quat {
        self.rot
    }
}

impl CanBeTransformed3d for SpotOn3d {
    type ProjectedVersion = SpotOnCurve;
    fn apply_transform3d(&self, t: &Transform3d) -> Self {
        let loc = self.loc.apply_transform3d(t);
        // filter for the rotation ones
        let rotations = t.rotation_transform().rotation_quat();
        let rot = rotations * self.rot;

        Self { loc, rot }
    }

    fn project(&self) -> SpotOnCurve {
        let dir = self.tangent();
        let angle = Angle::new(dir.y.atan2(dir.x));

        SpotOnCurve::new(self.loc.project(), angle)
    }
}

impl CanBeTransformed3d for Vec<SpotOn3d> {
    type ProjectedVersion = Vec<SpotOnCurve>;
    fn apply_transform3d(&self, t: &Transform3d) -> Self {
        self.iter().map(|x| x.apply_transform3d(t)).collect_vec()
    }

    fn project(&self) -> Vec<SpotOnCurve> {
        self.iter().map(|x| x.project()).collect_vec()
    }
}

pub trait ToVec3 {
    fn to_vec3(&self) -> Vec3;
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

#[derive(Clone, Debug, Copy)]
pub struct CubicBezier3d {
    start: SpotOn3d,
    start_strength: f32,
    end: SpotOn3d,
    end_strength: f32,
}
impl CubicBezier3d {
    pub fn new(start: SpotOn3d, start_strength: f32, end: SpotOn3d, end_strength: f32) -> Self {
        Self {
            start,
            start_strength,
            end,
            end_strength,
        }
    }

    pub fn at(&self, pct: f32) -> SpotOn3d {
        let start = self.start;
        let end = self.end;
        let start_s = self.start_strength;
        let end_s = self.end_strength;

        let s = (end.loc() - start.loc()).length();
        let p0 = start.loc();
        let p1 = start.loc() + start.tangent() * s * start_s;
        let p2 = end.loc() - end.tangent() * s * end_s;
        let p3 = end.loc();
        eval_3d_bezier(p0, p1, p2, p3, pct)
    }

    pub fn frames(&self, n: usize) -> Vec<SpotOn3d> {
        let mut spot = self.start;
        let mut out = Vec::with_capacity(n + 1);
        out.push(spot);
        for i in 1..=n {
            let next = self.at(i as f32 / n as f32);
            spot = spot.change_dir(next.tangent()).at_loc(next.loc());
            out.push(spot);
        }
        out
    }

    pub fn start(&self) -> SpotOn3d {
        self.start
    }
}
