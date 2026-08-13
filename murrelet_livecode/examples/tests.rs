use std::collections::HashMap;

use glam::*;
use lerpable::Lerpable;
use murrelet_common::*;
use murrelet_livecode::cachedcompute::CachedCompute;
use murrelet_livecode::{types::AdditionalContextNode, unitcells::*};
use murrelet_livecode_derive::Cached;
use murrelet_livecode::livecode::LivecodeToControl;
use murrelet_livecode_derive::{Livecode, LivecodeOnly, NestEdit};

#[derive(Debug, Clone, Livecode, Lerpable, Default)]
pub struct BasicTypes {
    a_number: f32,
    c_vec2: Vec2,
    b_angle: AnglePi,
    c_vec3: Vec3,
    b_color: MurreletColor,
    something: Vec<f32>,
    list_of_vec2: Vec<Vec2>,
    option_f32: Option<f32>,
    // option_vec2: Option<Vec2>,
    a_usize: usize,
    list_of_usize: Vec<usize>,
    list_of_u32: Vec<u32>,
}

fn empty_string() -> String {
    String::new()
}

fn empty_string_lazy() -> String {
    String::new()
}

#[derive(Debug, Clone, Livecode, Lerpable, Default)]
pub struct BasicTypesWithDefaults {
    #[livecode(serde_default = "zeros")]
    a_number: f32,
    b_color: MurreletColor,
    #[livecode(serde_default = "0")]
    c_vec2_serde_default: Vec2,
    something: Vec<f32>,
    list_of_vec2: Vec<Vec2>,
    #[livecode(kind = "none", serde_default = "empty_string")]
    label: String,
    #[livecode(kind = "none")]
    #[lerpable(method = "skip")]
    b: HashMap<String, String>,
    list_test: Vec<BasicTypes>,
}

#[derive(Debug, Clone, Livecode, Lerpable, Default)]
struct TestLazy {
    #[lerpable(method = "skip")]
    lazy: LazyBasicTypes,
    // lazy_test: LazyBasicTypes,
}

// vec elements are expanded (vec_repeat/gated) at control->world time, which blends,
// so a Vec<LazyT> field needs LazyT: Lerpable. lazy things step rather than blend.
impl Lerpable for LazyTestLazy {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        lerpable::step(self, other, pct)
    }
}

#[derive(Debug, Clone, Livecode, Lerpable, Default)]
struct TestVecOfLazy {
    #[lerpable(method = "skip")]
    lazies: Vec<LazyTestLazy>,
}

#[derive(Debug, Clone, Livecode, Lerpable, Default)]
enum EnumTest {
    #[default]
    A,
    B(TestLazy),
    C(#[lerpable(method = "skip")] LazyTestLazy),
    D(Vec<f32>),
    E(f32),
    F(#[lerpable(method = "skip")] Vec<LazyTestLazy>),
}

#[derive(Debug, Clone, Livecode, Lerpable, Default)]
struct TestNewType(Vec<EnumTest>);

// #[derive(Debug, Clone, Livecode, Lerpable, Default)]
// struct SequencerTest {
//     sequencer: SimpleSquareSequence,
//     ctx: AdditionalContextNode,
//     #[livecode(src = "sequencer", ctx = "ctx")]
//     node: UnitCells<TestNewType>,
//     #[livecode(src = "sequencer", ctx = "ctx")]
//     #[lerpable(method = "skip")]
//     node_two: UnitCells<LazyBasicTypes>,
// }

// fn make_grid(
//     x: usize,
//     y: usize,
//     cell_size: Vec2,
//     offset_alternating: bool,
// ) -> Vec<UnitCellContext> {
//     let x_usize = x;
//     let y_usize = y;

//     (0..x_usize)
//         .flat_map(|x| {
//             let x_idx = IdxInRange::new(x, x_usize);
//             (0..y_usize).map(move |y| {
//                 let y_idx = IdxInRange::new(y, y_usize);
//                 let idx = IdxInRange2d::new_from_idx(x_idx, y_idx);
//                 let ctx = UnitCellIdx::from_idx2d(idx, 1.0);

//                 let mut center = if offset_alternating {
//                     let mut center = idx.to_alternating_i().center_of_cell();
//                     center += vec2(-0.5, 0.0);
//                     if idx.i.i() % 2 == 1 {
//                         center += vec2(0.5, 0.5);
//                     }

//                     let offset_angle = AnglePi::new(1.0 / 6.0);
//                     let diag_scale = offset_angle.to_norm_dir() * cell_size.x / (100.0 * 0.5);

//                     center *= diag_scale;
//                     center
//                 } else {
//                     let mut center = idx.center_of_cell();
//                     center *= vec2(cell_size.x, cell_size.y) / 100.0;
//                     center
//                 };

//                 center *= 100.0;

//                 let transform = SimpleTransform2d::new(vec![
//                     SimpleTransform2dStep::translate(center),
//                     SimpleTransform2dStep::scale_both(cell_size.x / 100.0),
//                 ]);

//                 UnitCellContext::new(ctx, transform)
//             })
//         })
//         .collect::<Vec<_>>()
// }

// #[derive(Clone, Debug, Default, Livecode, Lerpable)]
// pub struct SimpleSquareSequence {
//     rows: usize,
//     cols: usize,
//     size: f32,
// }
// impl UnitCellCreator for SimpleSquareSequence {
//     fn to_unit_cell_ctxs(&self) -> Vec<UnitCellContext> {
//         make_grid(self.cols, self.rows, vec2(self.size, self.size), false)
//     }
// }

// // new type

#[derive(Clone, Debug, Default, Livecode, Lerpable)]
pub struct NewTypeWithType(f32);

#[derive(Clone, Debug, Default, Livecode, Lerpable)]
pub struct NewTypeWithTypeVec2(Vec2);

#[derive(Clone, Debug, Default, Livecode, Lerpable)]
pub struct NewTypeWithVec(Vec<f32>);

#[derive(Clone, Debug, Default, LivecodeOnly)]
pub struct NewTypeWithStruct(BasicTypes);

#[derive(Clone, Debug, Default, LivecodeOnly)]
pub struct NewTypeWithStructLazy(LazyBasicTypes);

// #[derive(Debug, Clone, Cached)]
// pub struct BirdOutline {

//     back: CachedCompute<f32>,
//     neck_back: CachedCompute<Vec<Vec2>>,

// }

fn main() {}

#[cfg(test)]
mod vec_of_lazy_tests {
    use super::*;
    use murrelet_livecode::expr::MixedEvalDefs;
    use murrelet_livecode::lazy::IsLazy;
    use murrelet_livecode::livecode::LivecodeFromWorld;

    fn conf(yaml: &str) -> TestVecOfLazy {
        let c: ControlTestVecOfLazy = serde_yaml::from_str(yaml).unwrap();
        c.o_dummy().unwrap()
    }

    #[test]
    fn stays_lazy_through_the_world_struct() {
        let conf = conf(
            r#"
lazies:
  - lazy: {a_number: "1.0", c_vec2: ["0", "0"], b_angle: "0", c_vec3: ["0","0","0"], b_color: ["0","0","0","1"], something: [], list_of_vec2: [], option_f32: null, a_usize: "0", list_of_usize: [], list_of_u32: []}
"#,
        );
        assert_eq!(conf.lazies.len(), 1);

        let ctx = MixedEvalDefs::new();
        let evaled = conf.lazies[0].eval_lazy(&ctx).unwrap();
        assert_eq!(evaled.lazy.eval_lazy(&ctx).unwrap().a_number, 1.0);
    }

    #[test]
    fn vec_repeat_expands_lazy_elements() {
        let conf = conf(
            r#"
lazies:
  - repeat: "3"
    prefix: "j"
    what:
      - lazy: {a_number: "j_i", c_vec2: ["0", "0"], b_angle: "0", c_vec3: ["0","0","0"], b_color: ["0","0","0","1"], something: [], list_of_vec2: [], option_f32: null, a_usize: "0", list_of_usize: [], list_of_u32: []}
"#,
        );
        assert_eq!(conf.lazies.len(), 3);
    }
}

#[cfg(test)]
mod nestedit_container_tests {
    use super::*;
    use murrelet_livecode::nestedit::{NestEditable, NestedMod};

    fn mods(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn apply<T: NestEditable>(x: &T, pairs: &[(&str, &str)]) -> T {
        let m = mods(pairs);
        x.nest_update(NestedMod::from_dict(&m))
    }

    fn sample() -> BasicTypesWithDefaults {
        BasicTypesWithDefaults {
            something: vec![1.0, 2.0, 3.0],
            list_of_vec2: vec![vec2(0.0, 0.0), vec2(5.0, 6.0)],
            label: "hi".to_owned(),
            b: mods(&[("k", "v"), ("other", "w")]),
            list_test: vec![
                BasicTypes {
                    a_number: 1.0,
                    ..Default::default()
                },
                BasicTypes {
                    a_number: 2.0,
                    option_f32: Some(7.0),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    // BUG-M316: these all used to silently return the original value.
    #[test]
    fn vec_of_primitives() {
        let got = apply(&sample(), &[("something.1", "9.5")]);
        assert_eq!(got.something, vec![1.0, 9.5, 3.0]);
    }

    #[test]
    fn vec_of_structs_reaches_nested_leaf() {
        let got = apply(&sample(), &[("list_test.1.a_number", "9.5")]);
        assert_eq!(got.list_test[0].a_number, 1.0, "sibling untouched");
        assert_eq!(got.list_test[1].a_number, 9.5);
    }

    #[test]
    fn vec_of_structs_reaches_through_a_second_container() {
        let got = apply(&sample(), &[("list_test.0.list_of_vec2.0.x", "4")]);
        assert_eq!(got.list_test[0].list_of_vec2, Vec::<Vec2>::new());

        let mut base = sample();
        base.list_test[0].list_of_vec2 = vec![vec2(1.0, 2.0)];
        let got = apply(&base, &[("list_test.0.list_of_vec2.0.x", "4")]);
        assert_eq!(got.list_test[0].list_of_vec2[0], vec2(4.0, 2.0));
    }

    #[test]
    fn vec_of_vec2_subfields() {
        let got = apply(&sample(), &[("list_of_vec2.1.y", "8")]);
        assert_eq!(got.list_of_vec2[1], vec2(5.0, 8.0));
        assert_eq!(got.list_of_vec2[0], vec2(0.0, 0.0));
    }

    #[test]
    fn option_is_transparent() {
        let got = apply(&sample(), &[("list_test.1.option_f32", "3.5")]);
        assert_eq!(got.list_test[1].option_f32, Some(3.5));
        // None stays None — there's nothing to descend into.
        assert_eq!(got.list_test[0].option_f32, None);
    }

    #[test]
    fn hashmap_keys() {
        let got = apply(&sample(), &[("b.k", "changed")]);
        assert_eq!(got.b.get("k").unwrap(), "changed");
        assert_eq!(got.b.get("other").unwrap(), "w");
    }

    #[test]
    fn unmatched_path_is_a_noop_not_a_panic() {
        let got = apply(&sample(), &[("something.99", "1"), ("b.nope", "1")]);
        assert_eq!(got.something, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn get_round_trips_the_write() {
        let got = apply(&sample(), &[("list_test.1.a_number", "9.5")]);
        assert_eq!(got.nest_getter("list_test.1.a_number").unwrap(), "9.5");
        assert_eq!(got.nest_getter("list_of_vec2.1.y").unwrap(), "6");
        assert_eq!(got.nest_getter("b.k").unwrap(), "\"v\"");
    }

    #[test]
    fn get_out_of_range_index_errors_instead_of_panicking() {
        assert!(sample().nest_getter("something.99").is_err());
        assert!(sample().nest_getter("something.notanumber").is_err());
    }
}
