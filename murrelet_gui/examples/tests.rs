use murrelet_gui::{CanMakeGUI, MurreletGUI, MurreletGUISchema};

#[derive(MurreletGUI)]
pub struct BasicTypes {
    a_number: f32,
    b_number: f32,
    something: Vec<f32>,
    // s: String,
}

// #[derive(Clone)]
// struct CustomMethod {
//     pct: f64,
// }
// impl IsLerpingMethod for CustomMethod {
//     fn has_lerp_stepped(&self) -> bool {
//         false
//     }

//     fn partial_lerp_pct(&self, _i: usize, _total: usize) -> f64 {
//         0.0
//     }

//     fn lerp_pct(&self) -> f64 {
//         0.0
//     }

//     fn with_lerp_pct(&self, pct: f64) -> Self {
//         let mut c = self.clone();
//         c.pct = pct;
//         c
//     }
// }

// fn custom_method() -> CustomMethod {
//     CustomMethod { pct: 0.0 }
// }

// fn custom_func<T: IsLerpingMethod>(this: &f32, other: &f32, _pct: &T) -> f32 {
//     *this - *other
// }

// #[derive(Debug, Clone, MurreletUX)]
// pub struct BasicTypesWithOverrides {
//     a_number: f32,
//     something: Vec<f32>,
//     label: String,
//     b: HashMap<String, String>,
// }

// #[derive(Debug, Clone)]
// struct MurreletUXType();

// #[derive(Debug, Clone, MurreletUX)]
// enum EnumTest {
//     A,
//     B(BasicTypesWithOverrides),
// }

// #[derive(Debug, Clone)]
// struct SimpleNewtype(f32);
// impl MurreletUX for SimpleNewtype {
//     fn lerpify<T: IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
//         if pct.lerp_pct() > 0.25 {
//             self.clone()
//         } else {
//             other.clone()
//         }
//     }

//     fn lerp_partial<T: IsLerpingMethod>(&self, pct: T) -> Self {
//         SimpleNewtype(pct.lerp_pct() as f32)
//     }
// }

// #[derive(Debug, Clone, MurreletUX)]

fn main() {
    // let b = BasicTypes{
    //     a_number: 1.0,
    //     b_number: -10.0,
    // };
    let test_val = BasicTypes::make_gui();

    assert_eq!(test_val, MurreletGUISchema::Skip)
}
