// place to put newtypes for livecode

pub mod anglepi {
    use murrelet_common::{Angle, AnglePi, IsAngle};
    use murrelet_livecode_derive::Livecode;

    #[derive(Clone, Copy, Debug, Livecode, Default)]
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

    impl From<LivecodeAnglePi> for Angle {
        fn from(value: LivecodeAnglePi) -> Self {
            value._to_angle_pi().as_angle()
        }
    }
}
