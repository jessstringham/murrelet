use glam::*;
use itertools::Itertools;
use murrelet_common::*;
use murrelet_livecode::unitcells::{UnitCellContext, UnitCellCreator, UnitCellExprWorldContext};
use murrelet_livecode_derive::*;

const REFERENCE_SIZE: f32 = 100.0;

#[derive(Clone, Debug, Livecode)]
pub enum Sequencer {
    Square(SimpleSquareSequence),
    Rect(SimpleRectSequence),
    Hex(SimpleHexSequence),
}
impl UnitCellCreator for Sequencer {
    fn to_unit_cell_ctxs(&self) -> Vec<UnitCellContext> {
        match &self {
            Sequencer::Square(g) => g.to_unit_cell_ctxs(),
            Sequencer::Rect(g) => g.to_unit_cell_ctxs(),
            Sequencer::Hex(g) => g.to_unit_cell_ctxs(),
        }
    }
}

#[derive(Clone, Debug, Default, Livecode)]
pub struct SimpleHexSequence {
    rows: usize,
    cols: usize,
    size: f32,
}
impl UnitCellCreator for SimpleHexSequence {
    fn to_unit_cell_ctxs(&self) -> Vec<UnitCellContext> {
        make_grid(self.cols, self.rows, vec2(self.size, self.size), true)
    }
}

#[derive(Clone, Debug, Default, Livecode)]
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

#[derive(Clone, Debug, Default, Livecode)]
pub struct SimpleRectSequence {
    rows: usize,
    cols: usize,
    size: Vec2,
}

impl SimpleRectSequence {
    pub fn new(rows: usize, cols: usize, size: Vec2) -> Self {
        Self { rows, cols, size }
    }
}

impl UnitCellCreator for SimpleRectSequence {
    fn to_unit_cell_ctxs(&self) -> Vec<UnitCellContext> {
        make_grid(self.cols, self.rows, self.size, false)
    }
}

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
                    let diag_scale =
                        offset_angle.to_norm_dir() * cell_size.x / (REFERENCE_SIZE * 0.5);

                    center *= diag_scale;
                    center
                } else {
                    let mut center = idx.center_of_cell();
                    center *= vec2(cell_size.x, cell_size.y) / REFERENCE_SIZE;
                    center
                };

                center *= REFERENCE_SIZE;

                let transform = Mat3::from_translation(center)
                    * Mat3::from_scale(cell_size.x / 100.0 * Vec2::ONE);
                UnitCellContext::new(ctx, mat4_from_mat3_transform(transform))
            })
        })
        .collect_vec()
}
