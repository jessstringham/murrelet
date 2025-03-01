use murrelet_gen::{CanSampleFromDist, MurreletGen};

#[derive(Debug, MurreletGen)]
pub struct BasicTypes {
    #[murrelet_gen(method_f32(uniform(start = -10.0, end = 1.0 )))]
    a_number: f32,
    #[murrelet_gen(method_f32(uniform(start = -10.0, end = 1.0 )))]
    b_number: usize,
    // c_number: u64,
    // d_number: i32,
    // bool: bool,
    // something: Vec<f32>,
    // s: String,
    // #[murrelet_gui(reference = "test")]
    // referenced_string: String,
}

// fn custom_func() -> MurreletGenSchema {
//     MurreletGenSchema::Val(ValueGUI::Num)
// }

// #[derive(MurreletGen)]
// pub struct OverridesAndRecursive {
//     a_number: f32,
//     something: Vec<BasicTypes>,
//     #[murrelet_gui(func = "custom_func")]
//     label: String,
//     #[murrelet_gui(kind = "skip")]
//     b: HashMap<String, String>,
// }

// #[derive(MurreletGen)]
// enum EnumTest {
//     A,
//     B(OverridesAndRecursive),
// }

// #[derive(MurreletGen)]
// struct SimpleNewtype(f32);

//     fn lerp_partial<T: IsLerpingMethod>(&self, pct: T) -> Self {
//         SimpleNewtype(pct.lerp_pct() as f32)
//     }
// }

// #[derive(Debug, Clone, MurreletUX)]

fn main() {
    let seed = 42;

    let test_val = BasicTypes::gen_from_seed(seed);

    // println!("test_val {:?}", test_val);
    panic!("{:?}", test_val)

    // let basic_types_schema = MurreletGenSchema::Struct(vec![
    //     ("a_number".to_owned(), MurreletGenSchema::Val(ValueGUI::Num)),
    //     ("b_number".to_owned(), MurreletGenSchema::Val(ValueGUI::Num)),
    //     ("c_number".to_owned(), MurreletGenSchema::Val(ValueGUI::Num)),
    //     ("d_number".to_owned(), MurreletGenSchema::Val(ValueGUI::Num)),
    //     ("bool".to_owned(), MurreletGenSchema::Val(ValueGUI::Bool)),
    //     (
    //         "something".to_owned(),
    //         MurreletGenSchema::list(MurreletGenSchema::Val(ValueGUI::Num)),
    //     ),
    //     ("s".to_owned(), MurreletGenSchema::Skip),
    //     (
    //         "referenced_string".to_owned(),
    //         MurreletGenSchema::Val(ValueGUI::Name("test".to_owned())),
    //     ),
    // ]);

    // assert_eq!(test_val, basic_types_schema);

    // let test_val = OverridesAndRecursive::make_gui();

    // let overrides_and_recursive_schema = MurreletGenSchema::Struct(vec![
    //     ("a_number".to_owned(), MurreletGenSchema::Val(ValueGUI::Num)),
    //     (
    //         "something".to_owned(),
    //         MurreletGenSchema::list(basic_types_schema),
    //     ),
    //     ("label".to_owned(), MurreletGenSchema::Val(ValueGUI::Num)), // make sure it calls the override
    //     ("b".to_owned(), MurreletGenSchema::Skip),
    // ]);
    // assert_eq!(test_val, overrides_and_recursive_schema);

    // let test_val = EnumTest::make_gui();

    // assert_eq!(
    //     test_val,
    //     MurreletGenSchema::Enum(vec![
    //         (MurreletEnumValGUI::Unit("A".to_owned())),
    //         (MurreletEnumValGUI::Unnamed("B".to_owned(), overrides_and_recursive_schema)),
    //     ])
    // );
}
