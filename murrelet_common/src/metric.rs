//! metrics that can aggregate stats about f32s (averages, ranges) or Vec2
//! (like centers and width and heights)

use glam::{Vec2, vec2};

use crate::{
    Rect,
    polyline::{IsPolyline, Polyline},
};

impl Default for BoundMetric {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BoundMetricF32 {
    left: f32,
    right: f32,
    count: usize,
    sum: f32,
}
impl BoundMetricF32 {
    pub fn new() -> BoundMetricF32 {
        BoundMetricF32 {
            left: f32::MAX,
            right: f32::MIN,
            count: 0,
            sum: 0.0,
        }
    }

    pub fn add_point(&mut self, x: f32) {
        if x < self.left {
            self.left = x
        }
        if x > self.right {
            self.right = x
        }
        self.count += 1;
        self.sum += x;
    }

    pub fn center(&self) -> f32 {
        0.5 * (self.right + self.left)
    }

    pub fn size(&self) -> f32 {
        self.right - self.left
    }

    pub fn scale(&self) -> f32 {
        self.size()
    }

    pub fn min(&self) -> f32 {
        self.left
    }

    pub fn max(&self) -> f32 {
        self.right
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn avg(&self) -> f32 {
        self.sum / self.count as f32
    }
}

impl Default for BoundMetricF32 {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BoundMetricUsize {
    min: usize,
    max: usize,
    count: usize,
    sum: usize,
}
impl BoundMetricUsize {
    pub fn new() -> BoundMetricUsize {
        BoundMetricUsize {
            min: usize::MAX,
            max: usize::MIN,
            count: 0,
            sum: 0,
        }
    }

    pub fn new_init(x: usize) -> BoundMetricUsize {
        let mut a = Self::new();
        a.add_point(x);
        a
    }

    pub fn add_point(&mut self, x: usize) {
        if x < self.min {
            self.min = x
        }
        if x > self.max {
            self.max = x
        }
        self.count += 1;
        self.sum += x;
    }

    pub fn size(&self) -> usize {
        self.max - self.min
    }

    pub fn scale(&self) -> usize {
        self.size()
    }

    pub fn min(&self) -> usize {
        self.min
    }

    pub fn max(&self) -> usize {
        self.max
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for BoundMetricUsize {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BoundMetric {
    x_bound: BoundMetricF32,
    y_bound: BoundMetricF32,
}
impl BoundMetric {
    pub fn new() -> BoundMetric {
        BoundMetric {
            x_bound: BoundMetricF32::new(),
            y_bound: BoundMetricF32::new(),
        }
    }

    pub fn new_from_points(vs: &[Vec2]) -> BoundMetric {
        let mut n = Self::new();
        n.add_points(vs);
        n
    }

    pub fn add_polyline(&mut self, f: &Polyline) {
        for v in f.into_iter_vec2() {
            self.add_point(v);
        }
    }

    pub fn new_from_polyline(f: &Polyline) -> BoundMetric {
        let mut n = Self::new();
        for v in f.into_iter_vec2() {
            n.add_point(v);
        }
        n
    }

    pub fn new_from_many_polylines(f: &[Polyline]) -> BoundMetric {
        let mut n = Self::new();
        for v in f {
            n.add_polyline(v);
        }
        n
    }

    pub fn new_from_vec_vecs(vvs: &Vec<Vec<Vec2>>) -> BoundMetric {
        let mut n = Self::new();
        for v in vvs {
            n.add_points(v);
        }
        n
    }

    pub fn lower_left(&self) -> Vec2 {
        vec2(self.x_bound.left, self.y_bound.left)
    }

    pub fn upper_right(&self) -> Vec2 {
        vec2(self.x_bound.right, self.y_bound.right)
    }

    pub fn as_rect(&self) -> Rect {
        Rect::from_corners(self.lower_left(), self.upper_right())
    }

    pub fn add_points(&mut self, vs: &[Vec2]) {
        for v in vs {
            self.add_point(*v)
        }
    }

    pub fn add_point(&mut self, v: Vec2) {
        self.x_bound.add_point(v.x);
        self.y_bound.add_point(v.y);
    }

    pub fn center(&self) -> Vec2 {
        0.5 * (self.upper_right() + self.lower_left())
    }

    pub fn width(&self) -> f32 {
        self.x_bound.size()
    }

    pub fn height(&self) -> f32 {
        self.y_bound.size()
    }

    pub fn scale(&self) -> f32 {
        if self.width() > self.height() {
            self.width()
        } else {
            self.height()
        }
    }

    pub fn set_y_max(&mut self, min: f32) {
        self.y_bound.right = min
    }

    pub fn y_min(&self) -> f32 {
        self.y_bound.left
    }

    pub fn y_max(&self) -> f32 {
        self.y_bound.right
    }

    pub fn x_min(&self) -> f32 {
        self.x_bound.left
    }

    pub fn x_max(&self) -> f32 {
        self.x_bound.right
    }

    pub fn update_with_other(&mut self, other: &BoundMetric) {
        self.add_point(other.lower_left());
        self.add_point(other.upper_right());
    }

    pub fn min(&self) -> Vec2 {
        vec2(self.x_min(), self.y_min())
    }

    pub fn max(&self) -> Vec2 {
        vec2(self.x_max(), self.y_max())
    }
}
