// place to put newtypes for livecode

pub mod anglepi {
    use glam::Vec2;
    use murrelet_common::{AnglePi, IsAngle};
    use murrelet_livecode_derive::Livecode;

    #[derive(Clone, Copy, Debug, Livecode)]
    pub struct LivecodeAnglePi(f32);
    impl LivecodeAnglePi {
        pub const ZERO: Self = LivecodeAnglePi(0.0);

        fn _to_angle_pi(&self) -> AnglePi {
            AnglePi::new(self.0)
        }

        pub fn new<A: IsAngle>(f: A) -> Self {
            Self(f.angle_pi())
        }
    }
    impl IsAngle for LivecodeAnglePi {
        fn angle_pi(&self) -> f32 {
            self._to_angle_pi().angle_pi()
        }

        fn angle(&self) -> f32 {
            self._to_angle_pi().angle()
        }

        fn as_angle(&self) -> murrelet_common::Angle {
            self._to_angle_pi().as_angle()
        }

        fn as_angle_pi(&self) -> AnglePi {
            self._to_angle_pi().as_angle_pi()
        }

        fn to_norm_dir(&self) -> Vec2 {
            self._to_angle_pi().to_norm_dir()
        }

        fn to_mat3(&self) -> glam::Mat3 {
            self._to_angle_pi().to_mat3()
        }

        fn perp_to_left(&self) -> murrelet_common::Angle {
            self._to_angle_pi().perp_to_left()
        }

        fn perp_to_right(&self) -> murrelet_common::Angle {
            self._to_angle_pi().perp_to_right()
        }
    }
}
