// use std::collections::HashMap;

// use murrelet_gui::{CanChangeToGUI, CanMakeGUI, MurreletEnumValGUI, MurreletGUISchema, ValueGUI};
// use murrelet_schema::*;
// use murrelet_schema_derive::MurreletSchema;

// #[derive(MurreletSchema)]
// pub struct BasicTypes {
//     a_number: f32,
//     b_number: usize,
//     c_number: u64,
//     d_number: i32,
//     bool: bool,
//     something: Vec<f32>,
//     s: String,
//     referenced_string: String,
// }

// fn custom_func() -> MurreletSchema {
//     MurreletSchema::Val(murrelet_schema::MurreletPrimitive::Num)
// }

// #[derive(MurreletSchema)]
// pub struct OverridesAndRecursive {
//     a_number: f32,
//     something: Vec<BasicTypes>,
//     #[murrelet_schema(func = "custom_func")]
//     label: String,
//     #[murrelet_schema(kind = "skip")]
//     b: HashMap<String, String>,
// }

// #[derive(MurreletSchema)]
// enum EnumTest {
//     A,
//     B(OverridesAndRecursive),
// }

// #[derive(MurreletSchema)]
// struct SimpleNewtype(f32);

// //     fn lerp_partial<T: IsLerpingMethod>(&self, pct: T) -> Self {
// //         SimpleNewtype(pct.lerp_pct() as f32)
// //     }
// // }

// // #[derive(Debug, Clone, MurreletUX)]

// fn main() {
//     // let b = BasicTypes{
//     //     a_number: 1.0,
//     //     b_number: -10.0,
//     // };
//     let test_val = BasicTypes::make_schema().change_to_gui();

//     let basic_types_schema = MurreletGUISchema::Struct(
//         "BasicTypes".to_string(),
//         vec![
//             ("a_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
//             ("b_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
//             ("c_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
//             ("d_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
//             ("bool".to_owned(), MurreletGUISchema::Val(ValueGUI::Bool)),
//             (
//                 "something".to_owned(),
//                 MurreletGUISchema::list(MurreletGUISchema::Val(ValueGUI::Num)),
//             ),
//             ("s".to_owned(), MurreletGUISchema::Val(ValueGUI::String)),
//             (
//                 "referenced_string".to_owned(),
//                 MurreletGUISchema::Val(ValueGUI::String),
//             ),
//         ],
//     );

//     assert_eq!(test_val, basic_types_schema);

//     let test_val = OverridesAndRecursive::make_schema().change_to_gui();

//     let overrides_and_recursive_schema = MurreletGUISchema::Struct(
//         "OverridesAndRecursive".to_string(),
//         vec![
//             ("a_number".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)),
//             (
//                 "something".to_owned(),
//                 MurreletGUISchema::list(basic_types_schema),
//             ),
//             ("label".to_owned(), MurreletGUISchema::Val(ValueGUI::Num)), // make sure it calls the override
//             ("b".to_owned(), MurreletGUISchema::Skip),
//         ],
//     );
//     assert_eq!(test_val, overrides_and_recursive_schema);

//     let test_val = EnumTest::make_schema().change_to_gui();

//     assert_eq!(
//         test_val,
//         MurreletGUISchema::Enum(
//             "EnumTest".to_string(),
//             vec![
//                 (MurreletEnumValGUI::Unit("A".to_owned())),
//                 (MurreletEnumValGUI::Unnamed("B".to_owned(), overrides_and_recursive_schema)),
//             ],
//             false
//         )
//     );
// }
