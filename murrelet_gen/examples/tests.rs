use glam::Vec2;
use murrelet_common::MurreletColor;
use murrelet_gen::{CanSampleFromDist, MurreletGen};

// #[derive(Debug, MurreletGen)]
#[derive(Debug, MurreletGen)]
pub struct BasicTypes {
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    a_number: f32,
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    another_number: f32,
    #[murrelet_gen(method(f32_fixed(val = 0.23)))]
    fixed_number: f32,
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
    #[murrelet_gen(method(f32_uniform_pos_neg(start = 10.0, end = 30.0)))]
    uniform_pos_neg: f32,
    // #[murrelet_gen(method(f32_normal(mu = 10.0, sigma = 30.0)))]
    // normal: f32,
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

#[derive(Debug, MurreletGen)]
pub struct Tiny {
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    a_number: f32,
    #[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))]
    another_number: f32,
}

#[derive(Debug, MurreletGen)]
pub struct OverridesAndRecursive {
    #[murrelet_gen(method(recurse))]
    basic: Tiny,
    #[murrelet_gen(method(vec_length(min = 1, max = 4)))]
    #[murrelet_gen(method_inner(recurse))]
    something: Vec<Tiny>,
}

#[derive(Debug, MurreletGen)]
enum EnumTest {
    #[murrelet_gen(weight = 1.0)]
    A,
    #[murrelet_gen(weight = 1.0)]
    B(#[murrelet_gen(method(recurse))] Tiny),
    #[murrelet_gen(weight = 1.0)]
    C(#[murrelet_gen(method(recurse))] Tiny),
}

// #[derive(MurreletGen)]
// struct SimpleNewtype(#[murrelet_gen(method(f32_uniform(start = 0.0, end = 1.0)))] f32);

fn main() {
    println!("BasicTypes::rn_count() {:?}", BasicTypes::rn_count());

    println!("BasicTypes::rn_names() {:?}", BasicTypes::rn_names());

    let b = BasicTypes::gen_from_seed(1);

    println!("b {:?}", b);
    println!("BasicTypes::to_dist() {:?}", b.to_dist());

    println!("BasicTypes::to_dist_mask() {:?}", b.to_dist_mask());

    // // let c = Vec2::ONE;

    assert_eq!(BasicTypes::rn_count(), b.to_dist().len());
    assert_eq!(BasicTypes::rn_count(), b.to_dist_mask().len());

    println!("round trip {:?}", BasicTypes::sample_dist(&b.to_dist(), 0));

    for i in b.to_dist() {
        assert!(i >= 0.0);
        assert!(i <= 1.0);
    }

    let c = OverridesAndRecursive::gen_from_seed(13);

    println!("c {:?}", c);

    println!(
        "OverridesAndRecursive::rn_names() {:?}",
        OverridesAndRecursive::rn_names()
    );

    println!("OverridesAndRecursive::to_dist() {:?}", c.to_dist());
    println!(
        "OverridesAndRecursive::to_dist_mask() {:?}",
        c.to_dist_mask()
    );

    // println!("EnumTest::rn_names() {:?}", EnumTest::rn_names());

    assert_eq!(BasicTypes::rn_count(), BasicTypes::rn_names().len());
    assert_eq!(Tiny::rn_count(), Tiny::rn_names().len());
    assert_eq!(
        OverridesAndRecursive::rn_count(),
        OverridesAndRecursive::rn_names().len()
    );

    let d = EnumTest::gen_from_seed(13);

    assert_eq!(EnumTest::rn_count(), EnumTest::rn_names().len());

    println!("d {:?}", d);

    println!("EnumTest::rn_names() {:?}", EnumTest::rn_names());

    println!("EnumTest::to_dist() {:?}", d.to_dist());
    println!("EnumTest::to_dist_mask() {:?}", d.to_dist_mask());

    // // println!(
    // //     "OverridesAndRecursive::rn_count() {:?}",
    // //     OverridesAndRecursive::rn_count()
    // // );

    // assert_eq!(BasicTypes::rn_count(), 37);

    // println!(
    //     "OverridesAndRecursive::gen_from_seed(42) {:?}",
    //     OverridesAndRecursive::gen_from_seed(42)
    // );

    // assert_eq!(OverridesAndRecursive::rn_count(), 11);
    // assert_eq!(EnumTest::rn_count(), 7);

    // for seed in 32..43 {
    //     let test_val = BasicTypes::gen_from_seed(seed);

    //     // there's a small chance they will be equal, but we know for this seed they aren't
    //     assert!(test_val.a_number != test_val.another_number);
    //     assert!(test_val.another_number_wider_range > 1.0);
    //     assert_eq!(test_val.fixed_number, 0.23);

    //     assert!(test_val.something.len() > 3);
    //     assert!(test_val.something.len() <= 10);

    //     let test_val2 = OverridesAndRecursive::gen_from_seed(seed);
    //     assert!(test_val2.something.len() >= 1);
    //     assert!(test_val2.something.len() <= 4);

    //     // println!("test_val {:?}", test_val);

    //     let test_val = BasicTypes::gen_from_seed(seed);
    //     let test_val2 = OverridesAndRecursive::gen_from_seed(seed);
    //     let test_val3 = EnumTest::gen_from_seed(seed);

    //     println!("test_val {:?}", test_val);
    //     // println!("test_val2 {:?}", test_val2);
    //     // println!("test_val3 {:?}", test_val3);

    // for i in b.to_dist() {
    //     assert!(i >= 0.0);
    //     assert!(i < 1.0);
    // }

    // }
}
