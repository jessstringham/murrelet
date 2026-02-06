use glam::Vec2;
use murrelet_common::{Angle, IsAngle, SpotOnCurve};

use crate::svg::glam_to_lyon;

#[derive(Debug, Clone, Copy)]
pub struct CubicBezier {
    pub from: Vec2,
    pub ctrl1: Vec2,
    pub ctrl2: Vec2,
    pub to: Vec2,
}
impl CubicBezier {
    pub fn safe_from_spots_s(
        in_spot: SpotOnCurve,
        out_spot: SpotOnCurve,
        strength: Vec2,
    ) -> Option<Self> {
        if in_spot.loc.distance(out_spot.loc) < 1.0e-3 {
            None
        } else {
            Some(Self::from_spots_s(in_spot, out_spot, strength))
        }
    }

    pub fn from_spots_s(in_spot: SpotOnCurve, out_spot: SpotOnCurve, strength: Vec2) -> Self {
        Self::from_spots(in_spot, strength.x, out_spot, strength.y)
    }

    pub fn from_spots(
        in_spot: SpotOnCurve,
        in_strength: f32,
        out_spot: SpotOnCurve,
        out_strength: f32,
    ) -> Self {
        let norm_dist = in_spot.loc().distance(out_spot.loc());

        let ctrl1 = in_spot.loc() + in_spot.angle().to_norm_dir() * in_strength * norm_dist;
        let ctrl2 = out_spot.loc() + out_spot.angle().to_norm_dir() * out_strength * norm_dist;

        Self {
            from: in_spot.loc(),
            ctrl1,
            ctrl2,
            to: out_spot.loc(),
        }
    }

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

    pub fn loc_at_pct(&self, t: f32) -> Vec2 {
        let (a, _) = self.split(t);
        a.to
    }

    pub fn start_to_tangent(&self) -> (SpotOnCurve, f32) {
        let ctrl_line = self.from - self.ctrl1;
        let dir = Angle::new(ctrl_line.to_angle())
            .as_angle_pi()
            .normalize_angle();

        (
            SpotOnCurve {
                loc: self.from,
                angle: dir.into(),
            },
            ctrl_line.length(),
        )
    }

    pub fn end_to_tangent(&self) -> (SpotOnCurve, f32) {
        let ctrl_line = self.ctrl2 - self.to;
        let dir = Angle::new(ctrl_line.to_angle())
            .as_angle_pi()
            .normalize_angle();

        (
            SpotOnCurve {
                loc: self.to,
                angle: dir.into(),
            },
            ctrl_line.length(),
        )
    }

    pub fn tangent_at_pct(&self, pct: f32) -> SpotOnCurve {
        let (start, _) = self.split(pct);
        let (t, _a) = start.end_to_tangent();
        t
    }

    pub fn reverse(&self) -> CubicBezier {
        CubicBezier {
            from: self.to,
            ctrl1: self.ctrl2,
            ctrl2: self.ctrl1,
            to: self.from,
        }
    }

    pub fn tangent_at_pct_safe(&self, pct: f32) -> SpotOnCurve {
        if pct < 0.01 {
            self.start_to_tangent().0
        } else if pct > 0.99 {
            self.end_to_tangent().0
        } else {
            self.tangent_at_pct(pct)
        }
    }

    pub fn apply_vec2_tranform(&self, f: impl Fn(Vec2) -> Vec2) -> Self {
        Self {
            from: f(self.from),
            ctrl1: f(self.ctrl1),
            ctrl2: f(self.ctrl2),
            to: f(self.to),
        }
    }

    pub fn approx_length(&self) -> f32 {
        let lyon_cubic = lyon::geom::CubicBezierSegment {
            from: glam_to_lyon(self.from),
            ctrl1: glam_to_lyon(self.ctrl1),
            ctrl2: glam_to_lyon(self.ctrl2),
            to: glam_to_lyon(self.to),
        };
        lyon_cubic.approximate_length(0.1)
    }
}
