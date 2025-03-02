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
    #[murrelet_gen(method(string_choice(choices(a = 1.0, b = 1.0, c = 1.0))))]
    s: String,
}

// fn custom_func() -> MurreletGenSchema {
//     MurreletGenSchema::Val(ValueGUI::Num)
// }

#[derive(Debug, MurreletGen)]
pub struct OverridesAndRecursive {
    #[murrelet_gen(method(recurse))]
    basic: BasicTypes,
    #[murrelet_gen(method(vec_length(min = 1, max = 4)))]
    #[murrelet_gen(method_inner(recurse))]
    something: Vec<BasicTypes>,
    // #[murrelet_gen(method(string_choice(choices(a = 1.0, b = 1.0, c = 1.0))))]
    // label: String,
    // b: HashMap<String, String>,
}

#[derive(Debug, MurreletGen)]
pub struct Tiny {
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    a_number: f32,
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    another_number: f32,
}

#[derive(Debug, MurreletGen)]
enum EnumTest {
    #[murrelet_gen(weight = 1.0)]
    A,
    #[murrelet_gen(weight = 1.0)]
    B(#[murrelet_gen(method(recurse))] Tiny),
}

// #[derive(MurreletGen)]
// struct SimpleNewtype(f32);

fn main() {
    println!("BasicTypes::rn_count() {:?}", BasicTypes::rn_count());
    println!(
        "OverridesAndRecursive::rn_count() {:?}",
        OverridesAndRecursive::rn_count()
    );

    assert_eq!(BasicTypes::rn_count(), 33);

    assert_eq!(OverridesAndRecursive::rn_count(), 166);

    for seed in 32..43 {
        let test_val = BasicTypes::gen_from_seed(seed);

        // there's a small chance they will be equal, but we know for this seed they aren't
        assert!(test_val.a_number != test_val.another_number);
        assert!(test_val.another_number_wider_range > 1.0);

        assert!(test_val.something.len() > 3);
        assert!(test_val.something.len() <= 10);

        let test_val2 = OverridesAndRecursive::gen_from_seed(seed);
        assert!(test_val2.something.len() >= 1);
        assert!(test_val2.something.len() < 4);

        // println!("test_val {:?}", test_val);

        let test_val = BasicTypes::gen_from_seed(seed);
        let test_val2 = OverridesAndRecursive::gen_from_seed(seed);
        let test_val3 = EnumTest::gen_from_seed(seed);

        println!("test_val {:?}", test_val);
        // println!("test_val2 {:?}", test_val2);
        // println!("test_val3 {:?}", test_val3);
    }

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
