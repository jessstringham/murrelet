//! IdxInRange represents things like "item 3 of 7"

use glam::{vec2, Vec2};
use itertools::Itertools;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::lerp;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdxInRange {
    i: u64,
    total: u64, // the count
}
impl IdxInRange {
    pub fn new<T: TryInto<u64>, U: TryInto<u64>>(i: T, total: U) -> IdxInRange
    where
        <T as TryInto<u64>>::Error: core::fmt::Debug,
        <U as TryInto<u64>>::Error: core::fmt::Debug,
    {
        IdxInRange {
            i: i.try_into().expect("can't convert to u64"),
            total: total.try_into().expect("can't convert to u64"),
        }
    }

    pub fn enumerate<'a, T, I>(iter: I) -> Vec<(IdxInRange, T)>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let total = iter.len();
        iter.enumerate()
            .map(|(i, v)| (IdxInRange::new(i, total), v))
            .collect_vec()
    }

    pub fn prev_i(&self) -> Option<IdxInRange> {
        if self.i == 0 {
            None
        } else {
            Some(IdxInRange {
                i: self.i - 1,
                total: self.total,
            })
        }
    }

    pub fn next_i(&self) -> Option<IdxInRange> {
        if self.i + 1 >= self.total {
            None
        } else {
            Some(IdxInRange {
                i: self.i + 1,
                total: self.total,
            })
        }
    }

    pub fn last_i(&self) -> IdxInRange {
        IdxInRange {
            i: self.i - 1,
            total: self.total,
        }
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn total_usize(&self) -> usize {
        self.total.try_into().expect("can't convert to usize")
    }

    pub fn to_usize(&self) -> usize {
        self.i.try_into().expect("can't convert to usize")
    }

    pub fn half_step_pct(&self) -> f32 {
        0.5 / self.total as f32
    }

    // goes from 0 to 1 inclusive
    pub fn pct(&self) -> f32 {
        if self.total == 1 {
            // hm, could be either 0 or 1, so set to 0.5...
            0.5
        } else {
            self.i as f32 / (self.total - 1) as f32
        }
    }

    pub fn to_centered(&self, boundary: f32) -> f32 {
        (2.0 * self.pct() - 1.0) * boundary
    }

    pub fn to_range<T>(&self, start: T, end: T) -> T
    where
        T: std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
        f32: std::ops::Mul<T, Output = T>,
    {
        lerp(start, end, self.pct())
    }

    pub fn i(&self) -> u64 {
        self.i
    }

    fn is_last(&self) -> bool {
        self.i == self.total - 1
    }

    pub fn amount_from_end(&self) -> u64 {
        self.total - self.i - 1
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IdxInRange2d {
    pub i: IdxInRange,
    pub j: IdxInRange,
}
impl IdxInRange2d {
    pub fn new<T: TryInto<u64> + Copy, U: TryInto<u64> + Copy>(i: T, j: T, total: U) -> IdxInRange2d
    where
        <T as TryInto<u64>>::Error: core::fmt::Debug,
        <U as TryInto<u64>>::Error: core::fmt::Debug,
    {
        IdxInRange2d {
            i: IdxInRange::new(i, total),
            j: IdxInRange::new(j, total),
        }
    }

    pub fn to_alternating_i(&self) -> IdxInRange2d {
        IdxInRange2d {
            i: IdxInRange::new(self.i.i() / 2, self.i.total / 2),
            j: self.j,
        }
    }

    pub fn new_rect<T: TryInto<u64> + Copy, U: TryInto<u64> + Copy>(
        i: T,
        j: T,
        total_i: U,
        total_j: U,
    ) -> IdxInRange2d
    where
        <T as TryInto<u64>>::Error: core::fmt::Debug,
        <U as TryInto<u64>>::Error: core::fmt::Debug,
    {
        IdxInRange2d {
            i: IdxInRange::new(i, total_i),
            j: IdxInRange::new(j, total_j),
        }
    }

    // seed iterates across each item in the row
    pub fn new_from_idx(i: IdxInRange, j: IdxInRange) -> IdxInRange2d {
        IdxInRange2d { i, j }
    }

    pub fn pct(&self) -> Vec2 {
        vec2(self.i.pct(), self.j.pct())
    }

    pub fn totals_vec2(&self) -> Vec2 {
        vec2(self.i.total_usize() as f32, self.j.total_usize() as f32)
    }

    pub fn to_centered(&self, boundary: f32) -> (f32, f32) {
        (self.i.to_centered(boundary), self.j.to_centered(boundary))
    }

    pub fn to_centered_ij(&self, boundary_i: f32, boundary_j: f32) -> (f32, f32) {
        (
            self.i.to_centered(boundary_i),
            self.j.to_centered(boundary_j),
        )
    }

    pub fn to_centered_ij_vec(&self, boundary_i: f32, boundary_j: f32) -> Vec2 {
        let (x, y) = self.to_centered_ij(boundary_i, boundary_j);
        vec2(x, y)
    }

    pub fn to_seed(&self) -> u64 {
        self.i.i * self.i.total + self.j.i
    }

    pub fn to_rand(&self) -> f32 {
        let mut rng = StdRng::seed_from_u64(self.to_seed());
        rng.gen_range(0.0..1.0)
    }

    // centers this cell in a grid of cells
    pub fn center_of_cell(&self) -> Vec2 {
        let cell_idx = vec2(self.i.i as f32, self.j.i as f32);
        let centering_offset = -0.5 * self.totals_vec2();

        cell_idx + vec2(0.5, 0.5) + centering_offset
    }

    pub fn half_step_pct(&self) -> Vec2 {
        vec2(self.i.half_step_pct(), self.j.half_step_pct())
    }

    pub fn lerp_idx(&self, x: f32, y: f32) -> Vec<(usize, usize)> {
        // helps tell which indexes to use for lerping
        let x_idx = x as usize;
        let y_idx = y as usize;

        let x_is_too_far = x_idx + 1 >= self.i.total as usize;
        let y_is_too_far = y_idx + 1 >= self.j.total as usize;

        let a = (x_idx, y_idx);

        let (b, c, d) = match (x_is_too_far, y_is_too_far) {
            (false, false) => (
                (x_idx + 1, y_idx),
                (x_idx, y_idx + 1),
                (x_idx + 1, y_idx + 1),
            ),
            (true, false) => ((x_idx, y_idx), (x_idx, y_idx + 1), (x_idx, y_idx + 1)),
            (false, true) => ((x_idx + 1, y_idx), (x_idx, y_idx), (x_idx + 1, y_idx)),
            (true, true) => (a, a, a),
        };

        vec![a, b, c, d]
    }

    pub fn next_i(&self) -> Option<IdxInRange2d> {
        self.i.next_i().map(|i| IdxInRange2d { i, j: self.j })
    }

    pub fn is_alternate(&self) -> bool {
        (self.i.i() % 2 != 0) ^ (self.j.i() % 2 == 0)
    }

    pub fn is_last_x(&self) -> bool {
        self.i.is_last()
    }

    pub fn is_last_y(&self) -> bool {
        self.j.is_last()
    }

    pub fn to_ranges_ij(&self, domain: Vec2, range: Vec2) -> Vec2 {
        let x = self.i.to_range(domain.x, domain.y);
        let y = self.j.to_range(range.x, range.y);
        vec2(x, y)
    }
}
