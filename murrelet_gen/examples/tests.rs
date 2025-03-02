use glam::Vec2;
use murrelet_common::MurreletColor;
use murrelet_gen::{CanSampleFromDist, MurreletGen};

#[derive(Debug, MurreletGen)]
pub struct BasicTypes {
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    a_number: f32,
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    another_number: f32,
    #[murrelet_gen(method(f32_uniform(start = 1.0, end = 100.0)))]
    another_number_wider_range: f32,
    #[murrelet_gen(method(f32_uniform(start = -10.0, end = 0.0 )))]
    a_neg_number: f32,
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 30.0)))]
    a_usize: usize,
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 30.0)))]
    c_number: u64,
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 30.0)))]
    d_number: i32,
    #[murrelet_gen(method(bool_binomial(pct = 0.3)))]
    bool: bool,
    #[murrelet_gen(method(vec2_uniform_grid(x = 0.0, y = 0.0, width = 100.0, height = 100.0)))]
    xy: Vec2,
    #[murrelet_gen(method(vec2_circle(x = 0.0, y = 0.0, radius = 100.0)))]
    other_xy: Vec2,
    #[murrelet_gen(method(color_normal))]
    normal_color: MurreletColor, // 3 rn
    #[murrelet_gen(method(color_transparency))]
    transparent_color: MurreletColor, // 4 rn

    #[murrelet_gen(method(vec_length(min = 4, max = 10)))]
    #[murrelet_gen(method_inner(f32_uniform(start = 0.0, end = 1.0)))]
    something: Vec<f32>,
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
    for seed in 32..43 {
        let test_val = BasicTypes::gen_from_seed(seed);

        println!("BasicTypes::rn_count() {:?}", BasicTypes::rn_count());

        assert!(BasicTypes::rn_count() == 30);

        // there's a small chance they will be equal, but we know for this seed they aren't
        assert!(test_val.a_number != test_val.another_number);
        assert!(test_val.another_number_wider_range > 1.0);

        assert!(test_val.something.len() > 3);
        assert!(test_val.something.len() <= 10);

        // println!("test_val {:?}", test_val);
    }

    let test_val = BasicTypes::gen_from_seed(42);

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
