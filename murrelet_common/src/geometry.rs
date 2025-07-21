//! Some geometry-related types and helpers

use std::{f32::consts::PI, ops::Add};

use glam::{vec2, Mat3, Mat4, Vec2};

use crate::{
    intersection::{find_intersection_inf, within_segment},
    transform::TransformVec2,
    SimpleTransform2d, SimpleTransform2dStep,
};

pub fn a_pi(a: f32) -> AnglePi {
    AnglePi::new(a)
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct AnglePi(f32);
impl AnglePi {
    pub const ZERO: Self = AnglePi(0.0);

    pub fn new(v: f32) -> AnglePi {
        AnglePi(v)
    }

    pub fn to_transform(&self) -> SimpleTransform2d {
        SimpleTransform2d::rotate_pi(self.angle_pi())
    }

    pub fn _angle(&self) -> f32 {
        let a: Angle = (*self).into();
        a.angle()
    }

    pub fn _angle_pi(&self) -> f32 {
        self.0
    }

    pub fn scale(&self, scale: f32) -> Self {
        AnglePi(self._angle_pi() * scale)
    }

    pub fn abs(&self) -> Self {
        AnglePi(self.0.abs())
    }

    pub fn normalize_angle(&self) -> Self {
        let mut angle = self.angle_pi();
        angle %= 2.0;
        if angle >= 1.0 {
            angle -= 2.0;
        } else if angle < -1.0 {
            angle += 2.0;
        }

        AnglePi(angle)
    }

    pub fn is_neg(&self) -> bool {
        self.0 < 0.0
    }

    pub fn transform_vec2(&self, v: Vec2) -> Vec2 {
        SimpleTransform2d::rotate_pi(self.angle_pi())
            .to_mat3()
            .transform_vector2(v)
    }
}

impl std::ops::Neg for AnglePi {
    type Output = AnglePi;

    fn neg(self) -> Self::Output {
        AnglePi(-self.0)
    }
}

impl<A> std::ops::Add<A> for Angle
where
    A: Into<Angle>,
{
    type Output = Angle;

    fn add(self, rhs: A) -> Self::Output {
        let rhs_angle: Angle = rhs.into();

        Angle(self.0 + rhs_angle.0)
    }
}

impl std::ops::Neg for Angle {
    type Output = Angle;

    fn neg(self) -> Self::Output {
        Angle(-self.0)
    }
}

impl<A> std::ops::Sub<A> for Angle
where
    A: Into<Angle>,
{
    type Output = Angle;

    fn sub(self, rhs: A) -> Self::Output {
        let rhs_angle: Angle = rhs.into();

        Angle(self.0 - rhs_angle.0)
    }
}

impl std::ops::Mul<f32> for Angle {
    type Output = Angle;

    fn mul(self, rhs: f32) -> Self::Output {
        Angle::new(self.0 * rhs)
    }
}

impl<A> std::ops::Add<A> for AnglePi
where
    A: Into<Angle>,
{
    type Output = AnglePi;

    fn add(self, rhs: A) -> Self::Output {
        let lhs_angle: Angle = self.into();

        (lhs_angle + rhs).into()
    }
}

impl std::ops::Mul<f32> for AnglePi {
    type Output = AnglePi;

    fn mul(self, rhs: f32) -> Self::Output {
        (Angle::from(self) * rhs).into()
    }
}

impl<A> std::ops::Sub<A> for AnglePi
where
    A: Into<Angle>,
{
    type Output = AnglePi;

    fn sub(self, rhs: A) -> Self::Output {
        let lhs_angle: Angle = self.into();

        (lhs_angle - rhs).into()
    }
}

impl From<Angle> for AnglePi {
    fn from(value: Angle) -> Self {
        AnglePi(value.0 / PI)
    }
}

impl From<AnglePi> for Angle {
    fn from(value: AnglePi) -> Self {
        Angle(value.0 * PI)
    }
}

// newtype
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct Angle(f32);
impl Angle {
    pub fn new(v: f32) -> Angle {
        Angle(v)
    }

    pub fn _angle_pi(&self) -> f32 {
        let a: AnglePi = (*self).into();
        a._angle_pi()
    }

    pub fn scale(&self, s: f32) -> Self {
        Angle(self.angle() * s)
    }

    pub fn hyp_given_opp<L: IsLength>(&self, opp: L) -> Length {
        Length(opp.len() / self.angle().sin())
    }

    pub fn _angle(&self) -> f32 {
        self.0
    }

    // normalized direction
    fn _to_norm_dir(&self) -> Vec2 {
        let (s, c) = self.0.sin_cos();
        vec2(c, s)
    }

    // if a line is going along this angle, what line would be perp to the left?
    pub fn _perp_to_left(&self) -> Angle {
        self.add(AnglePi(0.5))
    }

    pub fn _perp_to_right(&self) -> Angle {
        self.add(AnglePi(-0.5))
    }

    pub fn transform(&self, transform: Mat4) -> Angle {
        // hmm, only care about rotating and scaling, drop the transforms
        let [x_axis, y_axis, z_axis, _] = transform.to_cols_array_2d();

        let [_, _, _, w_axis] = Mat4::IDENTITY.to_cols_array_2d();

        let t = Mat4::from_cols_array_2d(&[x_axis, y_axis, z_axis, w_axis]);

        Angle::new(t.transform_vec2(self.to_norm_dir()).to_angle())
    }

    pub fn is_vertical(&self) -> bool {
        (self.angle_pi() - 0.5 % 1.0) < 1e-2
    }

    pub fn is_horizontal(&self) -> bool {
        (self.angle_pi() - 0.0 % 1.0) < 1e-2
    }

    // todo: mirror across angle
}

impl std::fmt::Debug for Angle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        AnglePi::fmt(&(*self).into(), f)
    }
}

pub trait IsAngle {
    fn angle_pi(&self) -> f32;
    fn angle(&self) -> f32;
    fn as_angle(&self) -> Angle;
    fn as_angle_pi(&self) -> AnglePi;
    fn to_norm_dir(&self) -> Vec2;
    fn to_mat3(&self) -> Mat3;
    fn perp_to_left(&self) -> Angle;
    fn perp_to_right(&self) -> Angle;
}

impl<A> IsAngle for A
where
    Angle: From<A>,
    A: Copy,
{
    fn to_norm_dir(&self) -> Vec2 {
        Angle::from(*self)._to_norm_dir()
    }

    fn perp_to_left(&self) -> Angle {
        Angle::from(*self)._perp_to_left()
    }

    fn perp_to_right(&self) -> Angle {
        Angle::from(*self)._perp_to_right()
    }

    fn to_mat3(&self) -> Mat3 {
        Mat3::from_angle(Angle::from(*self).angle())
    }

    fn angle_pi(&self) -> f32 {
        Angle::from(*self)._angle_pi()
    }

    fn angle(&self) -> f32 {
        Angle::from(*self)._angle()
    }

    fn as_angle_pi(&self) -> AnglePi {
        AnglePi::from(Angle::from(*self))
    }

    fn as_angle(&self) -> Angle {
        (*self).into()
    }
}

// LENGTH

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Default)]
pub struct Length(f32);

impl Length {
    pub fn new(v: f32) -> Length {
        Length(v)
    }

    pub fn scale(&self, scale: f32) -> Length {
        Length(self.len() + scale)
    }

    pub fn abs(&self) -> Self {
        Length(self.len().abs())
    }

    pub fn minus(&self) -> Length {
        Length(-self.len())
    }
}

impl<A> std::ops::Sub<A> for Length
where
    A: IsLength,
{
    type Output = Length;

    fn sub(self, rhs: A) -> Self::Output {
        let other = rhs.len();

        Length(self.0 - other)
    }
}

impl<A> std::ops::Add<A> for Length
where
    A: IsLength,
{
    type Output = Length;

    fn add(self, rhs: A) -> Self::Output {
        let other = rhs.len();

        Length(self.0 + other)
    }
}

impl IsLength for Length {
    fn len(&self) -> f32 {
        self.0
    }

    fn to_length(&self) -> Length {
        *self
    }
}

pub trait IsLength {
    fn len(&self) -> f32;
    fn to_length(&self) -> Length;
}

impl IsLength for f32 {
    fn len(&self) -> f32 {
        *self
    }

    fn to_length(&self) -> Length {
        Length::new(*self)
    }
}

impl IsLength for PointToPoint {
    fn len(&self) -> f32 {
        self.start.distance(self.end)
    }

    fn to_length(&self) -> Length {
        Length::new(self.len())
    }
}

// Special types

// should combine this with Tangent...

#[derive(Debug, Copy, Clone)]
pub struct SpotOnCurve {
    loc: Vec2,
    angle: Angle,
}

impl SpotOnCurve {
    pub fn new<A: IsAngle>(loc: Vec2, angle: A) -> Self {
        Self {
            loc,
            angle: angle.as_angle(),
        }
    }

    pub fn loc(&self) -> Vec2 {
        self.loc
    }

    pub fn angle(&self) -> Angle {
        self.angle
    }

    pub fn transform(&self, m: Mat4) -> Self {
        SpotOnCurve::new(m.transform_vec2(self.loc), self.angle.transform(m))
    }

    pub fn to_right_vector(&self, length: f32) -> PointToPoint {
        PointToPoint::new(
            self.loc(),
            self.loc() + self.angle().perp_to_right().to_norm_dir() * length,
        )
    }

    pub fn find_intersection_tangent(&self, segment: PointToPoint) -> Option<Vec2> {
        // let fake_spot = self.loc() + self.angle.perp_to_left().to_norm_dir() * 100.0;
        let tangent_segment = self.to_right_vector(10.0);
        let intersection = tangent_segment.find_intersection_inf(segment);

        if let Some(i) = intersection {
            if segment.within_segment(i, 1.0e-4) {
                return intersection;
            }
        }
        None
    }

    pub fn to_line<L: IsLength>(&self, length: L) -> LineFromVecAndLen {
        LineFromVecAndLen::new(self.loc, self.angle, length.to_length())
    }

    pub fn turn_left_perp(&self) -> Self {
        Self {
            loc: self.loc,
            angle: self.angle.perp_to_left(),
        }
    }

    pub fn turn_right_perp(&self) -> Self {
        Self {
            loc: self.loc,
            angle: self.angle.perp_to_right(),
        }
    }

    pub fn move_left_perp_dist<L: IsLength>(&self, length: L) -> Vec2 {
        self.turn_left_perp()
            .to_line(length.to_length())
            .to_last_point()
    }

    pub fn move_right_perp_dist<L: IsLength>(&self, length: L) -> Vec2 {
        self.turn_right_perp()
            .to_line(length.to_length())
            .to_last_point()
    }

    pub fn rotate(&self, rotate: AnglePi) -> Self {
        Self {
            loc: self.loc,
            angle: self.angle + rotate,
        }
    }

    pub fn to_transform(&self) -> SimpleTransform2d {
        SimpleTransform2d::new(vec![
            SimpleTransform2dStep::rotate_pi(self.angle()),
            SimpleTransform2dStep::translate(self.loc()),
        ])
    }

    pub fn set_angle<A: Into<Angle>>(&self, new: A) -> Self {
        let mut c = self.clone();
        c.angle = new.into();
        c
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CornerAngleToAngle {
    point: Vec2,
    in_angle: Angle, // exact, not pi
    out_angle: Angle,
}

impl CornerAngleToAngle {
    pub fn new<A: Into<Angle>>(point: Vec2, in_angle: A, out_angle: A) -> Self {
        Self {
            point,
            in_angle: in_angle.into(),
            out_angle: out_angle.into(),
        }
    }

    // dist is how far away from the current point. left is positive (inside of angle) (i think)
    pub fn corner_at_point<L: IsLength>(&self, dist: L) -> Vec2 {
        // mid-way between the two angles, and then go perpindicular at some point

        let p = if dist.len() < 0.0 {
            AnglePi(0.5)
        } else {
            AnglePi(-0.5)
        };

        let target_angle = (self.out_angle + self.in_angle).scale(0.5) + p;
        let new_angle: Angle = target_angle - self.in_angle;

        let target_angle_norm_dir = target_angle.to_norm_dir();
        let new_length = new_angle.hyp_given_opp(dist);

        self.point + new_length.len() * target_angle_norm_dir
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PrevCurrNextVec2 {
    prev: Vec2,
    curr: Vec2,
    next: Vec2,
}
impl PrevCurrNextVec2 {
    pub fn new(prev: Vec2, curr: Vec2, next: Vec2) -> Self {
        Self { prev, curr, next }
    }

    pub fn angle(&self) -> Angle {
        let curr_to_prev = self.prev - self.curr;
        let next_to_curr = self.next - self.curr;
        Angle::new(curr_to_prev.angle_to(next_to_curr))
    }

    pub fn tri_contains(&self, other_v: &Vec2) -> bool {
        // https://nils-olovsson.se/articles/ear_clipping_triangulation/
        let v0 = self.next - self.prev;
        let v1 = self.curr - self.prev;
        let v2 = *other_v - self.prev;

        let dot00 = v0.dot(v0);
        let dot01 = v0.dot(v1);
        let dot02 = v0.dot(v2);
        let dot11 = v1.dot(v1);
        let dot12 = v1.dot(v2);

        let denom = dot00 * dot11 - dot01 * dot01;
        if denom.abs() < 1e-10 {
            return true;
        }

        let inv_denom = 1.0 / denom;
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

        (u >= 0.0) && (v >= 0.0) && (u + v < 1.0)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PointToPoint {
    start: Vec2,
    end: Vec2,
}
impl PointToPoint {
    pub fn new(start: Vec2, end: Vec2) -> Self {
        Self { start, end }
    }

    pub fn to_norm_dir(&self) -> Vec2 {
        (self.end - self.start).normalize()
    }

    // angle relative to 0
    pub fn angle(&self) -> Angle {
        Angle(self.to_norm_dir().to_angle())
    }

    pub fn midpoint(&self) -> Vec2 {
        // 0.5 * (self.start + self.end)
        self.pct(0.5)
    }

    pub fn to_vec(&self) -> Vec<Vec2> {
        vec![self.start, self.end]
    }

    pub fn to_tuple(&self) -> (Vec2, Vec2) {
        (self.start, self.end)
    }

    pub fn find_intersection_inf(self, other: PointToPoint) -> Option<Vec2> {
        find_intersection_inf(self.to_tuple(), other.to_tuple())
    }

    pub fn within_segment(self, intersection: Vec2, eps: f32) -> bool {
        within_segment(self.to_tuple(), intersection, eps)
    }

    pub fn start(&self) -> Vec2 {
        self.start
    }

    pub fn end(&self) -> Vec2 {
        self.end
    }

    pub fn closest_pt_to_pt(&self, intersection: Vec2) -> PointToPoint {
        let closest_point = self.closest_point_to_line(intersection);

        PointToPoint {
            start: intersection,
            end: closest_point,
        }
    }

    // drop a right angle down from intersection, where does it fall along
    // the line extended from point to point?
    pub fn closest_point_to_line(&self, intersection: Vec2) -> Vec2 {
        self.start + (intersection - self.start).dot(self.to_norm_dir()) * self.to_norm_dir()
    }

    pub fn pct(&self, loc: f32) -> Vec2 {
        self.start + loc * (self.end - self.start)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LineFromVecAndLen {
    start: Vec2,
    angle: Angle,
    length: Length,
}
impl LineFromVecAndLen {
    pub fn new<L: IsLength>(start: Vec2, angle: Angle, length: L) -> Self {
        Self {
            start,
            angle,
            length: length.to_length(),
        }
    }

    pub fn to_last_point(&self) -> Vec2 {
        self.start + self.length.len() * self.angle.to_norm_dir()
    }
}

// more curve things

//https://en.wikibooks.org/wiki/Algorithm_Implementation/Geometry/Tangents_between_two_circles

pub enum TangentBetweenCirclesKind {
    RightRight,
    LeftLeft,
    RightLeft,
    LeftRight,
}

pub fn tangents_between_two_circles(
    kind: TangentBetweenCirclesKind,
    v1: Vec2,
    r1: f32,
    v2: Vec2,
    r2: f32,
) -> Option<(Vec2, Vec2)> {
    let d_sq = v1.distance_squared(v2);

    // these are too close together
    if d_sq <= (r1 - r2).powi(2) {
        return None;
    }

    let d = d_sq.sqrt();

    let v = (v2 - v1) / d;

    let (sign1, sign2) = match kind {
        TangentBetweenCirclesKind::RightRight => (1.0, 1.0),
        TangentBetweenCirclesKind::LeftLeft => (1.0, -1.0),
        TangentBetweenCirclesKind::RightLeft => (-1.0, 1.0),
        TangentBetweenCirclesKind::LeftRight => (-1.0, -1.0),
    };

    let c = (r1 - sign1 * r2) / d;

    // Now we're just intersecting a line with a circle: v*n=c, n*n=1

    if c.powi(2) > 1.0 {
        return None;
    }
    let h = (1.0 - c.powi(2)).max(0.0).sqrt();

    let nx = v.x * c - sign2 * h * v.y;
    let ny = v.y * c + sign2 * h * v.x;

    let start = vec2(v1.x + r1 * nx, v1.y + r1 * ny);

    let end = vec2(v2.x + sign1 * r2 * nx, v2.y + sign1 * r2 * ny);

    Some((start, end))
}

#[derive(Copy, Clone, Debug)]
pub struct Tangent {
    pub loc: Vec2,
    pub dir: AnglePi,
    // strength: f32,
}

impl Tangent {
    pub fn dir(&self) -> AnglePi {
        self.dir
    }

    pub fn loc(&self) -> Vec2 {
        self.loc
    }
}
