// place to put newtypes for livecode

pub mod anglepi {
    use std::ops::Sub;

    use glam::Vec2;
    use lerpable::Lerpable;
    use murrelet_common::{Angle, AnglePi, IsAngle};
    use murrelet_gui::CanMakeGUI;
    use murrelet_livecode::lazy::ControlLazyNodeF32;
    use murrelet_livecode_derive::Livecode;

    use crate::transform2d::Transform2d;

    #[derive(Clone, Copy, Debug, Livecode, Lerpable, Default, PartialEq)]
    pub struct LivecodeAnglePi(f32);
    impl LivecodeAnglePi {
        pub const ZERO: Self = LivecodeAnglePi(0.0);

        fn _to_angle_pi(&self) -> AnglePi {
            AnglePi::new(self.0)
        }

        pub fn new<A: IsAngle>(f: A) -> Self {
            Self(f.angle_pi())
        }

        pub fn from_angle_pi(angle_pi: f32) -> Self {
            Self(angle_pi)
        }

        pub fn scale(&self, scale: f32) -> Self {
            Self(self.0 * scale)
        }

        pub fn transform_vec2(&self, v: glam::Vec2) -> Vec2 {
            Transform2d::rotate(self.angle_pi())
                .to_mat3()
                .transform_vector2(v)
        }
    }

    impl From<LivecodeAnglePi> for Angle {
        fn from(value: LivecodeAnglePi) -> Self {
            value._to_angle_pi().as_angle()
        }
    }

    impl From<LivecodeAnglePi> for AnglePi {
        fn from(value: LivecodeAnglePi) -> Self {
            value._to_angle_pi()
        }
    }

    impl CanMakeGUI for LivecodeAnglePi {
        fn make_gui() -> murrelet_gui::MurreletGUISchema {
            murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Angle)
        }
    }

    impl Sub for LivecodeAnglePi {
        type Output = Self;

        fn sub(self, other: Self) -> Self::Output {
            let new_angle = self.0 - other.0;
            LivecodeAnglePi(new_angle)
        }
    }

    impl ControlLazyLivecodeAnglePi {
        pub fn from_angle(a: AnglePi) -> Self {
            Self(ControlLazyNodeF32::Float(a.angle_pi()))
        }
    }
}
