// use std::collections::HashMap;

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
// }

// fn custom_func() -> MurreletSchema {
//     MurreletSchema::Val(MurreletPrimitive::Num)
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

// // //     fn lerp_partial<T: IsLerpingMethod>(&self, pct: T) -> Self {
// // //         SimpleNewtype(pct.lerp_pct() as f32)
// // //     }
// // // }

// // // #[derive(Debug, Clone, MurreletUX)]

// fn main() {
//     // let b = BasicTypes{
//     //     a_number: 1.0,
//     //     b_number: -10.0,
//     // };
//     let test_val = BasicTypes::make_schema();

//     let basic_types_schema = MurreletSchema::Struct(
//         "BasicTypes".to_string(),
//         vec![
//             (
//                 "a_number".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::Num),
//             ),
//             (
//                 "b_number".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::Num),
//             ),
//             (
//                 "c_number".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::Num),
//             ),
//             (
//                 "d_number".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::Num),
//             ),
//             (
//                 "bool".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::Bool),
//             ),
//             (
//                 "something".to_owned(),
//                 MurreletSchema::list(MurreletSchema::Val(MurreletPrimitive::Num)),
//             ),
//             (
//                 "s".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::String),
//             ),
//         ],
//     );

//     assert_eq!(test_val, basic_types_schema);

//     let test_val = OverridesAndRecursive::make_schema();

//     let overrides_and_recursive_schema = MurreletSchema::Struct(
//         "OverridesAndRecursive".to_string(),
//         vec![
//             (
//                 "a_number".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::Num),
//             ),
//             (
//                 "something".to_owned(),
//                 MurreletSchema::list(basic_types_schema),
//             ),
//             (
//                 "label".to_owned(),
//                 MurreletSchema::Val(MurreletPrimitive::Num),
//             ), // make sure it calls the override
//             ("b".to_owned(), MurreletSchema::Skip),
//         ],
//     );
//     assert_eq!(test_val, overrides_and_recursive_schema);

//     let test_val = EnumTest::make_schema();

//     assert_eq!(
//         test_val,
//         MurreletSchema::Enum(
//             "EnumTest".to_string(),
//             vec![
//                 (MurreletEnumVal::Unit("A".to_owned())),
//                 (MurreletEnumVal::Unnamed("B".to_owned(), overrides_and_recursive_schema)),
//             ],
//             false
//         )
//     );
// }

fn main() {}
