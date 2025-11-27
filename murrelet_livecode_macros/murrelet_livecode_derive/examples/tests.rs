use std::collections::HashMap;

use glam::*;
use lerpable::Lerpable;
use murrelet_common::*;
use murrelet_livecode::{types::AdditionalContextNode, unitcells::*};
use murrelet_livecode_derive::{Livecode, LivecodeOnly, NestEdit};

#[derive(Debug, Clone, Livecode, Lerpable, Default)]
pub struct BasicTypes {
    // a_number: f32,
    // #[lerpable(func = "lerpify_vec2")]
    // c_vec2: Vec2,
    // #[lerpable(func = "lerpify_vec3")]
    // c_vec3: Vec3,
    // b_color: MurreletColor,
    // something: Vec<f32>,
    #[lerpable(func = "lerpify_vec_vec2")]
    list_of_vec2: Vec<Vec2>,
    // option_f32: Option<f32>,
    // option_vec2: Option<Vec2>,
}

// fn empty_string() -> String {
//     String::new()
// }

// fn empty_string_lazy() -> String {
//     String::new()
// }

// #[derive(Debug, Clone, Livecode, Lerpable, Default)]
// pub struct BasicTypesWithDefaults {
//     #[livecode(serde_default = "zeros")]
//     a_number: f32,
//     b_color: MurreletColor,
//     #[livecode(serde_default = "0")]
//     #[lerpable(func = "lerpify_vec2")]
//     c_vec2: Vec2,
//     something: Vec<f32>,
//     // // #[lerpable(func = "lerpify_vec_vec2")]
//     // // list_of_vec2: Vec<Vec2>,
//     #[livecode(kind = "none", serde_default = "empty_string")]
//     label: String,
//     #[livecode(kind = "none")]
//     #[lerpable(method = "skip")]
//     b: HashMap<String, String>,
// }

// #[derive(Debug, Clone, Livecode, Lerpable, Default)]
// struct TestLazy {
//     #[lerpable(method = "skip")]
//     lazy: LazyBasicTypes,
// }

// #[derive(Debug, Clone, Livecode, Lerpable, Default)]
// enum EnumTest {
//     #[default]
//     A,
//     B(TestLazy),
//     C(#[lerpable(method = "skip")] LazyTestLazy),
// }

// #[derive(Debug, Clone, Livecode, Lerpable, Default)]
// struct TestNewType(Vec<EnumTest>);

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

// #[derive(Clone, Debug, Default, Livecode, Lerpable)]
// pub struct NewTypeWithType(f32);

// #[derive(Clone, Debug, Default, Livecode, Lerpable)]
// pub struct NewTypeWithTypeVec2(#[lerpable(func = "lerpify_vec2")] Vec2);

// #[derive(Clone, Debug, Default, Livecode, Lerpable)]
// pub struct NewTypeWithVec(Vec<f32>);

// #[derive(Clone, Debug, Default, LivecodeOnly)]
// pub struct NewTypeWithStruct(BasicTypes);

fn main() {}
