use glam::Vec2;
use murrelet_common::{Angle, IsAngle, Tangent};

#[derive(Debug, Clone)]
pub struct CubicBezier {
    pub from: Vec2,
    pub ctrl1: Vec2,
    pub ctrl2: Vec2,
    pub to: Vec2,
}
impl CubicBezier {
    pub fn new(from: Vec2, ctrl1: Vec2, ctrl2: Vec2, to: Vec2) -> Self {
        Self {
            from,
            ctrl1,
            ctrl2,
            to,
        }
    }

    pub fn line(from: Vec2, to: Vec2) -> Self {
        Self {
            from,
            ctrl1: from,
            ctrl2: to,
            to,
        }
    }

    pub fn split(&self, t: f32) -> (CubicBezier, CubicBezier) {
        let mid1 = self.from.lerp(self.ctrl1, t);
        let mid2 = self.ctrl1.lerp(self.ctrl2, t);
        let mid3 = self.ctrl2.lerp(self.to, t);

        let mid12 = mid1.lerp(mid2, t);
        let mid23 = mid2.lerp(mid3, t);

        let mid123 = mid12.lerp(mid23, t);

        (
            CubicBezier {
                from: self.from,
                ctrl1: mid1,
                ctrl2: mid12,
                to: mid123,
            },
            CubicBezier {
                from: mid123,
                ctrl1: mid23,
                ctrl2: mid3,
                to: self.to,
            },
        )
    }

    pub fn start_to_tangent(&self) -> (Tangent, f32) {
        // let side_len = self.from.distance(self.to);

        let ctrl_line = self.from - self.ctrl1;
        let dir = Angle::new(ctrl_line.to_angle())
            .as_angle_pi()
            .normalize_angle();

        // let strength = ctrl_line.length() / side_len;

        (
            Tangent {
                loc: self.from,
                dir: dir.into(),
                // strength,
            },
            ctrl_line.length(),
        )
    }

    pub fn end_to_tangent(&self) -> (Tangent, f32) {
        // let side_len = self.from.distance(self.to);

        let ctrl_line = self.ctrl2 - self.to;
        let dir = Angle::new(ctrl_line.to_angle())
            .as_angle_pi()
            .normalize_angle();

        // let strength = ctrl_line.length() / side_len;

        (
            Tangent {
                loc: self.to,
                dir: dir.into(),
                // strength,
            },
            ctrl_line.length(),
        )
    }

    pub fn tangent_at_pct(&self, pct: f32) -> Tangent {
        let (start, _) = self.split(pct);
        let (t, _a) = start.end_to_tangent();
        t
    }
}
