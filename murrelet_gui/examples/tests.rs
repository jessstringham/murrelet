use std::collections::HashMap;

use murrelet_gui::{CanMakeGUI, MurreletEnumValGUI, MurreletGUI, MurreletGUISchema, ValueGUI};

#[derive(MurreletGUI)]
pub struct BasicTypes {
    a_number: f32,
    b_number: usize,
    c_number: u64,
    d_number: i32,
    something: Vec<f32>,
    s: String,
    #[murrelet_gui(reference = "test")]
    referenced_string: String,
}

#[derive(MurreletGUI)]
pub struct OverridesAndRecursive {
    a_number: f32,
    something: Vec<BasicTypes>,
    label: String,
    #[murrelet_gui(kind = "skip")]
    b: HashMap<String, String>,
}

#[derive(MurreletGUI)]
enum EnumTest {
    A,
    B(OverridesAndRecursive),
}

#[derive(MurreletGUI)]
struct SimpleNewtype(f32);

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

    let basic_types_schema = MurreletGUISchema::Struct(vec![
        ("a_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
        ("b_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
        ("c_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
        ("d_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
        (
            "something".to_owned(),
            MurreletGUISchema::list(MurreletGUISchema::Val(ValueGUI::Num)),
        ),
        ("s".to_owned(), MurreletGUISchema::Skip),
        (
            "referenced_string".to_owned(),
            MurreletGUISchema::Val(ValueGUI::Name("test".to_owned())),
        ),
    ]);

    assert_eq!(test_val, basic_types_schema);

    let test_val = OverridesAndRecursive::make_gui();

    let overrides_and_recursive_schema = MurreletGUISchema::Struct(vec![
        ("a_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
        (
            "something".to_owned(),
            MurreletGUISchema::list(basic_types_schema),
        ),
        ("label".to_owned(), MurreletGUISchema::Skip),
        ("b".to_owned(), MurreletGUISchema::Skip),
    ]);
    assert_eq!(test_val, overrides_and_recursive_schema);

    let test_val = EnumTest::make_gui();

    assert_eq!(
        test_val,
        MurreletGUISchema::Enum(vec![
            (MurreletEnumValGUI::Unit("A".to_owned())),
            (MurreletEnumValGUI::Unnamed("B".to_owned(), overrides_and_recursive_schema)),
        ])
    );
}
