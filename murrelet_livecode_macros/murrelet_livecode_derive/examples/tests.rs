use glam::*;
use murrelet_common::*;
use murrelet_livecode::unitcells::*;
use murrelet_livecode_derive::{Livecode, UnitCell};

#[derive(Debug, Clone, Livecode, UnitCell, Default)]
struct SomethingElse {
    a_number: f32,
    b_color: MurreletColor,
    c_vec2: Vec2,
}

#[derive(Debug, Clone, Livecode, UnitCell, Default)]
enum EnumTest {
    #[default]
    A,
    B(SomethingElse),
}

#[derive(Debug, Clone, Livecode, UnitCell, Default)]
struct TestNewType(Vec<EnumTest>);

#[derive(Debug, Clone, Livecode)]
struct SequencerTest {
    sequencer: SimpleSquareSequence,
    ctx: UnitCellCtx,
    #[livecode(src = "sequencer", ctx = "ctx")]
    node: UnitCells<TestNewType>,
}

fn main() {}

fn make_grid(
    x: usize,
    y: usize,
    cell_size: Vec2,
    offset_alternating: bool,
) -> Vec<UnitCellContext> {
    let x_usize = x;
    let y_usize = y;

    (0..x_usize)
        .flat_map(|x| {
            let x_idx = IdxInRange::new(x, x_usize);
            (0..y_usize).map(move |y| {
                let y_idx = IdxInRange::new(y, y_usize);
                let idx = IdxInRange2d::new_from_idx(x_idx, y_idx);
                let ctx = UnitCellExprWorldContext::from_idx2d(idx, 1.0);

                let mut center = if offset_alternating {
                    let mut center = idx.to_alternating_i().center_of_cell();
                    center += vec2(-0.5, 0.0);
                    if idx.i.i() % 2 == 1 {
                        center += vec2(0.5, 0.5);
                    }

                    let offset_angle = AnglePi::new(1.0 / 6.0);
                    let diag_scale = offset_angle.to_norm_dir() * cell_size.x / (100.0 * 0.5);

                    center *= diag_scale;
                    center
                } else {
                    let mut center = idx.center_of_cell();
                    center *= vec2(cell_size.x, cell_size.y) / 100.0;
                    center
                };

                center *= 100.0;

                let transform = Mat3::from_translation(center)
                    * Mat3::from_scale(cell_size.x / 100.0 * Vec2::ONE);
                UnitCellContext::new(ctx, mat4_from_mat3_transform(transform))
            })
        })
        .collect::<Vec<_>>()
}

#[derive(Clone, Debug, Default, Livecode, UnitCell)]
pub struct SimpleSquareSequence {
    rows: usize,
    cols: usize,
    size: f32,
}
impl UnitCellCreator for SimpleSquareSequence {
    fn to_unit_cell_ctxs(&self) -> Vec<UnitCellContext> {
        make_grid(self.cols, self.rows, vec2(self.size, self.size), false)
    }
}
