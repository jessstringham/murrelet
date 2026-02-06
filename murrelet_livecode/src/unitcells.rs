use glam::*;
use itertools::Itertools;
use lerpable::{Lerpable, step};
use murrelet_common::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use std::fmt::Debug;
use std::{any::Any, collections::HashMap, fmt};

use crate::expr::{ExprWorldContextValues, IntoExprWorldContext, MixedEvalDefsRef};
use crate::livecode::LivecodeFromWorld;
use crate::state::LivecodeWorldState;
use crate::types::AdditionalContextNode;
use crate::types::LivecodeResult;

// helps you translate between LiveCode and UnitCells
pub struct TmpUnitCells<CtxSource: UnitCellCreator, Target> {
    sequencer: CtxSource,
    node: Box<dyn LivecodeFromWorld<Target>>,
    ctx: Option<MixedEvalDefsRef>,
    prefix: String,
}

impl<CtxSource: UnitCellCreator, Target: Default> TmpUnitCells<CtxSource, Target> {
    pub fn new(
        sequencer: CtxSource,
        node: Box<dyn LivecodeFromWorld<Target>>,
        ctx: Option<AdditionalContextNode>,
        prefix: &str,
    ) -> Self {
        Self {
            sequencer,
            node,
            ctx: ctx.map(MixedEvalDefsRef::from_ctx_node),
            prefix: prefix.to_owned(),
        }
    }
}

impl<CtxSource, Target> TmpUnitCells<CtxSource, Target>
where
    CtxSource: UnitCellCreator,
    Target: Default + std::fmt::Debug + Clone,
{
    pub fn eval_with_ctx(
        &self,
        world_ctx: &LivecodeWorldState,
        unit_cell_ctx: Option<&MixedEvalDefsRef>,
    ) -> Vec<UnitCell<Target>> {
        // right now this one doesn't usually return an error because we do stuff
        // to avoid returning every time, should tidy up

        let mut is_first_error = true;
        self.sequencer
            .to_unit_cell_ctxs()
            .iter()
            .map(|ctx| {
                // this has the
                // - world (t, midi, audio)
                // - app-level ctx
                // - unit cell location
                // it doesn't have sequencer ctx yet, we'll add that next

                let unit_cell_world_ctx_result =
                    world_ctx.clone_to_unitcell(ctx, &self.prefix, unit_cell_ctx);

                // and evaluate with this!
                // todo can i use the result to clean this up
                let node = match unit_cell_world_ctx_result {
                    Ok(unit_cell_world_ctx) => match self.node.o(&unit_cell_world_ctx) {
                        Ok(n) => n,
                        Err(err) => {
                            if is_first_error {
                                println!("{}", err);
                                is_first_error = false;
                            }
                            Target::default()
                        }
                    },
                    Err(err) => {
                        if is_first_error {
                            println!("{}", err);
                            is_first_error = false;
                        }
                        Target::default()
                    }
                };

                UnitCell::new(node, ctx.clone())
            })
            .collect::<Vec<_>>()
    }

    pub fn o(&self, ctx: &LivecodeWorldState) -> LivecodeResult<UnitCells<Target>> {
        Ok(UnitCells::new(self.eval_with_ctx(ctx, self.ctx.as_ref())))
    }
}

/// for structs that can be used to generate a bunch of different contexts
/// e.g. Tiler, crystals
pub trait UnitCellCreator {
    fn to_unit_cell_ctxs(&self) -> Vec<UnitCellContext>;
}

#[derive(Debug, Clone)]
pub struct UnitCell<Target> {
    pub node: Box<Target>,
    pub detail: UnitCellContext,
}

impl<Target> UnitCell<Target> {
    pub fn new(node: Target, detail: UnitCellContext) -> Self {
        Self {
            node: Box::new(node),
            detail,
        }
    }

    // convenience for macros, just copy over the detail
    pub fn to_other_type<NewTarget>(&self, node: NewTarget) -> UnitCell<NewTarget> {
        UnitCell::<NewTarget>::new(node, self.detail.clone())
    }

    pub fn transform_vec2(&self, v: Vec2) -> Vec2 {
        self.detail.transform().transform_vec2(v)
    }

    pub fn transform(&self) -> SimpleTransform2d {
        self.detail.transform()
    }

    pub fn center(&self) -> Vec2 {
        self.transform_vec2(Vec2::ZERO)
    }

    pub fn top(&self) -> Vec2 {
        self.transform_vec2(vec2(0.0, 50.0))
    }

    pub fn bottom(&self) -> Vec2 {
        self.transform_vec2(vec2(0.0, -50.0))
    }

    pub fn bounds(&self) -> Rect {
        Rect::from_corners(
            self.transform().transform_vec2(vec2(-50.0, -50.0)),
            self.transform().transform_vec2(vec2(50.0, 50.0)),
        )
    }

    pub fn idx(&self) -> IdxInRange2d {
        self.detail.idx.to_idx2d()
    }

    pub fn is_alternate(&self) -> bool {
        self.idx().is_alternate()
    }
}
impl<Target: Default> Default for UnitCell<Target> {
    fn default() -> Self {
        Self {
            node: Default::default(),
            detail: UnitCellContext::new(
                UnitCellIdx::from_idx1d(IdxInRange::new(0, 1)),
                SimpleTransform2d::ident(),
            ),
        }
    }
}

impl<N: Lerpable> Lerpable for UnitCell<N> {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        let node = self.node.lerpify(&other.node, pct);

        let detail = self.detail.lerpify(&other.detail, pct);

        UnitCell::new(node, detail)
    }
}

#[derive(Clone, Debug, Default)]
pub struct UnitCells<Target: std::fmt::Debug + Clone + Default> {
    pub items: Vec<UnitCell<Target>>,
}

impl<Target: std::fmt::Debug + Clone + Default> UnitCells<Target> {
    pub fn new(items: Vec<UnitCell<Target>>) -> Self {
        Self { items }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, UnitCell<Target>> {
        self.items.iter()
    }

    pub fn x_y_z_max(&self) -> (u64, u64, u64) {
        // each should have the same, so grab the first
        if let Some(first) = self.items.first() {
            first.detail.idx.max()
        } else {
            (0, 0, 0)
        }
    }

    pub fn to_vec2d(&self) -> Vec<Vec<Option<UnitCell<Target>>>> {
        self.as_map().to_vec2d()
    }

    pub fn as_map(&self) -> UnitCellLookup<Target> {
        let mut hm = HashMap::new();
        for item in &self.items {
            hm.insert(
                (
                    item.detail.idx.x_i,
                    item.detail.idx.y_i,
                    item.detail.idx.z_i,
                ),
                item.clone(),
            );
        }
        UnitCellLookup::new(hm, self.x_y_z_max())
    }

    // assuming pattern doesn't reach outside, which it might. Might give a buffer.
    pub fn bounds(&self) -> Rect {
        let mut bounds = BoundMetric::new();
        for item in self.iter() {
            if item.detail.is_base() {
                bounds.add_points(&item.detail.rect_bound())
            }
        }
        bounds.as_rect()
    }

    pub fn get_tile_at_loc(&self, v: Vec2) -> Option<UnitCell<Target>> {
        for item in self.iter() {
            if item.bounds().contains(v) {
                return Some(item.clone());
            }
        }

        None
    }

    // conveninence method
    pub fn copy_nodes(&self) -> Vec<Target> {
        self.items.iter().map(|x| *x.node.clone()).collect_vec()
    }
}

impl<Target: std::fmt::Debug + Clone + Default> FromIterator<UnitCell<Target>>
    for UnitCells<Target>
{
    fn from_iter<I: IntoIterator<Item = UnitCell<Target>>>(iter: I) -> Self {
        let vec: Vec<_> = iter.into_iter().collect();
        UnitCells { items: vec }
    }
}

impl<N: Clone + Debug + Default + Lerpable> Lerpable for UnitCells<N> {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        Self::new(self.items.lerpify(&other.items, pct))
    }
}

#[derive(Clone, Debug)]
pub struct UnitCellLookup<Target: std::fmt::Debug + Clone> {
    data: HashMap<(u64, u64, u64), UnitCell<Target>>,
    maxes: (u64, u64, u64),
}

impl<Target: std::fmt::Debug + Clone> UnitCellLookup<Target> {
    pub fn new(data: HashMap<(u64, u64, u64), UnitCell<Target>>, maxes: (u64, u64, u64)) -> Self {
        Self { data, maxes }
    }

    pub fn x_y_z_max(&self) -> (u64, u64, u64) {
        self.maxes
    }

    pub fn dims(&self) -> Dim2d {
        let (x, y, _) = self.maxes;
        Dim2d::from_x_y(x, y)
    }

    pub fn to_vec2d(&self) -> Vec<Vec<Option<UnitCell<Target>>>> {
        let mut vs = vec![];

        if self.maxes.2 > 1 {
            println!("z is more than 1");
        }

        for y_i in 0..self.maxes.1 {
            let mut row = vec![];
            for x_i in 0..self.maxes.0 {
                // println!("x_i {:?}", x_i);
                // println!("y_i {:?}", y_i);
                row.push(self.data.get(&(x_i, y_i, 0)).cloned());
            }
            vs.push(row)
        }

        vs
    }

    pub fn force_get_ij(&self, i: usize, j: usize) -> &UnitCell<Target> {
        self.get_ij(i, j).unwrap()
    }

    pub fn force_get_ij_tuple(&self, ij: (usize, usize)) -> &UnitCell<Target> {
        self.get_ij_tuple(ij).unwrap()
    }

    pub fn get_ij_tuple(&self, ij: (usize, usize)) -> Option<&UnitCell<Target>> {
        let (i, j) = ij;
        self.get_ij(i, j)
    }

    pub fn get_ij(&self, i: usize, j: usize) -> Option<&UnitCell<Target>> {
        self.data.get(&(i as u64, j as u64, 0))
    }

    pub fn get_ij_neighbor(
        &self,
        i: usize,
        j: usize,
        neighbor: CellNeighbor,
    ) -> Option<&UnitCell<Target>> {
        match neighbor {
            CellNeighbor::Hex(HexCellNeighbor::Up) => self.get_ij(i, j + 1),
            CellNeighbor::Hex(HexCellNeighbor::UpLeft) => {
                let jj = if i.is_multiple_of(2) { j + 1 } else { j };
                self.get_ij(i - 1, jj)
            }
            CellNeighbor::Hex(HexCellNeighbor::DownLeft) => {
                let jj = if i.is_multiple_of(2) { j } else { j - 1 };
                self.get_ij(i - 1, jj)
            }
            CellNeighbor::Hex(HexCellNeighbor::Down) => self.get_ij(i, j - 1),
            CellNeighbor::Hex(HexCellNeighbor::DownRight) => {
                let jj = if i.is_multiple_of(2) { j } else { j - 1 };
                self.get_ij(i + 1, jj)
            }
            CellNeighbor::Hex(HexCellNeighbor::UpRight) => {
                let jj = if i.is_multiple_of(2) { j + 1 } else { j };
                self.get_ij(i + 1, jj)
            }

            // grid
            CellNeighbor::Grid(GridCellNeighbor::Up) => self.get_ij(i, j + 1),
            CellNeighbor::Grid(GridCellNeighbor::UpLeft) => self.get_ij(i + 1, j + 1),
            CellNeighbor::Grid(GridCellNeighbor::Left) => self.get_ij(i + 1, j),
            CellNeighbor::Grid(GridCellNeighbor::DownLeft) => self.get_ij(i + 1, j - 1),
            CellNeighbor::Grid(GridCellNeighbor::Down) => self.get_ij(i, j - 1),
            CellNeighbor::Grid(GridCellNeighbor::DownRight) => self.get_ij(i - 1, j - 1),
            CellNeighbor::Grid(GridCellNeighbor::Right) => self.get_ij(i - 1, j),
            CellNeighbor::Grid(GridCellNeighbor::UpRight) => self.get_ij(i - 1, j + 1),
        }
    }

    pub fn get_ij_node(&self, i: usize, j: usize) -> Option<Target> {
        self.data
            .get(&(i as u64, j as u64, 0))
            .map(|x| *(x.node).clone())
    }

    pub fn force_get_dim(&self, dim: &Dim2d) -> &UnitCell<Target> {
        self.force_get_ij(dim.i(), dim.j())
    }

    // this probably only works for rectangles...
    pub fn loc_to_tile_idx_and_offset<T: Debug + Clone + Default + Lerpable>(
        &self,
        v: Vec2,
    ) -> Option<IdxAndOffset> {
        // blah
        for uc in self.data.values() {
            if uc.bounds().contains(v) {
                let local_offset = v - uc.center();
                let scaled_offset = local_offset / uc.bounds().wh();
                return Some(IdxAndOffset {
                    ij: uc.idx(),
                    offset_i: scaled_offset.x,
                    offset_j: scaled_offset.y,
                });
            }
        }
        None
    }
}

pub struct IdxAndOffset {
    ij: IdxInRange2d,
    // how far in i and j they are
    offset_i: f32,
    offset_j: f32,
}
impl IdxAndOffset {
    pub fn lerp_idxes(&self) -> [(usize, usize); 4] {
        self.ij.lerp_idx()
    }

    pub fn offset_i(&self) -> f32 {
        self.offset_i
    }

    pub fn offset_j(&self) -> f32 {
        self.offset_j
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum HexCellNeighbor {
    Up,
    UpLeft,
    DownLeft,
    Down,
    DownRight,
    UpRight,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum GridCellNeighbor {
    Up,
    UpLeft,
    Left,
    DownLeft,
    Down,
    DownRight,
    Right,
    UpRight,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum CellNeighbor {
    Hex(HexCellNeighbor),
    Grid(GridCellNeighbor),
}
impl CellNeighbor {
    pub fn hex_up() -> Self {
        CellNeighbor::Hex(HexCellNeighbor::Up)
    }
    pub fn hex_up_left() -> Self {
        CellNeighbor::Hex(HexCellNeighbor::UpLeft)
    }
    pub fn hex_down_left() -> Self {
        CellNeighbor::Hex(HexCellNeighbor::DownLeft)
    }
    pub fn hex_down() -> Self {
        CellNeighbor::Hex(HexCellNeighbor::Down)
    }
    pub fn hex_down_right() -> Self {
        CellNeighbor::Hex(HexCellNeighbor::DownRight)
    }
    pub fn hex_up_right() -> Self {
        CellNeighbor::Hex(HexCellNeighbor::UpRight)
    }

    pub fn grid_up() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::Up)
    }
    pub fn grid_up_left() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::UpLeft)
    }
    pub fn grid_left() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::Left)
    }
    pub fn grid_down_left() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::DownLeft)
    }
    pub fn grid_down() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::Down)
    }
    pub fn grid_down_right() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::DownRight)
    }
    pub fn grid_right() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::Right)
    }
    pub fn grid_up_right() -> Self {
        CellNeighbor::Grid(GridCellNeighbor::UpRight)
    }
}

pub trait TileInfo: Debug + Any + Sync + Send {
    // hmm, not sure about this Any
    fn clone_box(&self) -> Box<dyn TileInfo>;
    fn face(&self) -> Vec<Vec2>;
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn TileInfo> {
    fn clone(&self) -> Box<dyn TileInfo> {
        self.clone_box()
    }
}

// eh, should make this easier...
#[derive(Debug, Clone)]
pub struct UnitCellContext {
    idx: UnitCellIdx,
    ctx: Option<ExprWorldContextValues>,
    pub detail: UnitCellDetails,
    pub tile_info: Option<Box<dyn TileInfo>>,
}
impl UnitCellContext {
    pub fn new(idx: UnitCellIdx, transform: SimpleTransform2d) -> UnitCellContext {
        UnitCellContext {
            idx,
            ctx: None,
            detail: UnitCellDetails::new(transform),
            tile_info: None,
        }
    }

    pub fn new_expr(
        idx: UnitCellIdx,
        ctx: ExprWorldContextValues,
        transform: SimpleTransform2d,
    ) -> UnitCellContext {
        UnitCellContext {
            idx,
            ctx: Some(ctx),
            detail: UnitCellDetails::new(transform),
            tile_info: None,
        }
    }

    pub fn new_full(
        idx: UnitCellIdx,
        ctx: ExprWorldContextValues,
        transform: SimpleTransform2d,
        adjust_transform: SimpleTransform2d,
    ) -> UnitCellContext {
        UnitCellContext {
            idx,
            ctx: Some(ctx),
            detail: UnitCellDetails::new_fancy(transform, adjust_transform, true),
            tile_info: None,
        }
    }

    pub fn new_with_base(ctx: UnitCellIdx, detail: UnitCellDetails) -> UnitCellContext {
        UnitCellContext {
            idx: ctx,
            ctx: None,
            detail,
            tile_info: None,
        }
    }

    pub fn set_tile_info(&mut self, tile_info: Box<dyn TileInfo>) {
        self.tile_info = Some(tile_info);
    }

    pub fn new_with_info(
        ctx: UnitCellIdx,
        detail: UnitCellDetails,
        tile_info: Box<dyn TileInfo>,
    ) -> UnitCellContext {
        UnitCellContext {
            idx: ctx,
            ctx: None,
            detail,
            tile_info: Some(tile_info),
        }
    }

    pub fn new_with_option_info(
        ctx: UnitCellIdx,
        detail: UnitCellDetails,
        tile_info: Option<Box<dyn TileInfo>>,
    ) -> UnitCellContext {
        UnitCellContext {
            idx: ctx,
            ctx: None,
            detail,
            tile_info,
        }
    }

    pub fn rect_for_face(&self) -> Rect {
        // todo, replace this with rect_bound
        let mut b = BoundMetric::new();
        b.add_points(&self.rect_bound());
        b.as_rect()
    }

    // just updates details...
    // applies the other transform _after_ current
    pub fn combine(&self, other: &UnitCellContext) -> UnitCellContext {
        let ctx = match (&self.ctx, &other.ctx) {
            (None, None) => self.ctx.clone(),
            (None, Some(that)) => Some(that.clone()),
            (Some(this), None) => Some(this.clone()),
            (Some(this), Some(that)) => Some(this.clone().combine(that.clone())),
        };

        UnitCellContext {
            idx: self.idx,
            ctx,
            detail: other.detail.as_wallpaper().unwrap().combine(&self.detail),
            tile_info: None,
        }
    }

    pub fn combine_keep_other_ctx(&self, other: &UnitCellContext) -> UnitCellContext {
        let ctx = match (&self.ctx, &other.ctx) {
            (None, None) => self.ctx.clone(),
            (None, Some(that)) => Some(that.clone()),
            (Some(this), None) => Some(this.clone()),
            (Some(this), Some(that)) => Some(that.clone().combine(this.clone())),
        };

        UnitCellContext {
            idx: self.idx,
            ctx,
            detail: other.detail.as_wallpaper().unwrap().combine(&self.detail),
            tile_info: None,
        }
    }

    pub fn ctx(&self) -> UnitCellIdx {
        self.idx
    }

    pub fn transform(&self) -> SimpleTransform2d {
        self.detail.transform()
    }

    pub fn idx(&self) -> IdxInRange2d {
        self.idx.to_idx2d()
    }

    pub fn rect_bound(&self) -> Vec<Vec2> {
        let face = if let Some(tile_info) = &self.tile_info {
            tile_info.face()
        } else {
            let val = 50.0;
            vec![
                vec2(-val, -val),
                vec2(val, -val),
                vec2(val, val),
                vec2(-val, val),
            ]
        };
        self.transform_with_skew(&face).clone_to_vec()
    }

    pub fn is_base(&self) -> bool {
        self.detail.is_base()
    }

    pub fn transform_with_skew_mat4(&self) -> SimpleTransform2d {
        self.detail.transform_with_skew_mat4()
    }

    pub fn transform_with_skew<F: Transformable>(&self, v: &F) -> F {
        v.transform_with(&self.detail.transform_with_skew_mat4())
    }

    pub fn transform_one_point_with_skew(&self, v: Vec2) -> Vec2 {
        v.transform_with(&self.detail.transform_with_skew_mat4())
    }

    pub fn transform_no_skew_one_point(&self, v: Vec2) -> Vec2 {
        // also does adjust shape..
        self.detail.transform_no_skew_one_point(v)
    }

    pub fn transform_no_skew<F: Transformable>(&self, v: &F) -> F {
        // also does adjust shape..
        v.transform_with(&self.detail.transform_no_skew_mat4())
    }

    pub fn transform_no_skew_mat4(&self) -> SimpleTransform2d {
        self.detail.transform_no_skew_mat4()
    }

    pub fn adjust_shape(&self) -> SimpleTransform2d {
        self.detail.adjust_shape()
    }
}

impl IntoExprWorldContext for UnitCellContext {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues {
        let mut ctx_vals = self.idx.as_expr_world_context_values();

        let locs = vec![vec2(-50.0, -50.0), vec2(50.0, -50.0), vec2(50.0, 50.0)]
            .into_iter()
            .map(|x| self.detail.transform_with_skew_mat4().transform_vec2(x))
            .collect_vec();

        let width = locs[1].distance(locs[0]);
        let height = locs[1].distance(locs[2]);

        ctx_vals.set_val("u_width", LivecodeValue::float(width));
        ctx_vals.set_val("u_height", LivecodeValue::float(height));

        if let Some(expr) = &self.ctx {
            ctx_vals = ctx_vals.combine(expr.clone());
        }

        ctx_vals
    }
}

impl Lerpable for UnitCellContext {
    fn lerpify<T: lerpable::IsLerpingMethod>(&self, other: &Self, pct: &T) -> Self {
        let ctx = self
            .ctx()
            .experimental_lerp(&other.ctx(), pct.lerp_pct() as f32);
        let detail = self
            .detail
            .experimental_lerp(&other.detail, pct.lerp_pct() as f32);
        let tile_info = step(&self.tile_info, &other.tile_info, pct);
        UnitCellContext::new_with_option_info(ctx, detail, tile_info)
    }
}

// world state for unit cell
#[derive(Copy, Clone, Debug)]
pub struct UnitCellIdx {
    x: f32,
    y: f32,
    z: f32,
    x_i: u64,
    y_i: u64,
    z_i: u64,
    total_x: u64,
    total_y: u64,
    total_z: u64,
    seed: f32,
    h_ratio: f32, // width is always 100, what is h
}
impl UnitCellIdx {
    // this just needs to be interesting.... not correct
    pub fn experimental_lerp(&self, other: &Self, pct: f32) -> Self {
        UnitCellIdx {
            x: self.x.lerpify(&other.x, &pct),
            y: self.y.lerpify(&other.y, &pct),
            z: self.z.lerpify(&other.z, &pct),
            x_i: self.x_i.lerpify(&other.x_i, &pct),
            y_i: self.y_i.lerpify(&other.y_i, &pct),
            z_i: self.z_i.lerpify(&other.z_i, &pct),
            total_x: self.total_x.lerpify(&other.total_x, &pct),
            total_y: self.total_y.lerpify(&other.total_y, &pct),
            total_z: self.total_z.lerpify(&other.total_z, &pct),
            seed: self.seed.lerpify(&other.seed, &pct),
            h_ratio: self.h_ratio.lerpify(&other.h_ratio, &pct),
        }
    }

    pub fn from_idx2d_and_actual_xy(xy: Vec2, idx: IdxInRange2d, h_ratio: f32) -> UnitCellIdx {
        UnitCellIdx {
            x: xy.x,
            y: xy.y,
            z: 0.0,
            x_i: idx.i.i(),
            y_i: idx.j.i(),
            z_i: 0,
            seed: idx.to_seed() as f32,
            total_x: idx.i.total(),
            total_y: idx.j.total(),
            total_z: 1,
            h_ratio,
        }
    }

    pub fn from_idx1d(idx: IdxInRange) -> UnitCellIdx {
        UnitCellIdx {
            x: idx.pct(),
            y: 0.0,
            z: 0.0,
            x_i: idx.i(),
            y_i: 0,
            z_i: 0,
            seed: idx.i() as f32,
            total_x: idx.total(),
            total_y: 1,
            total_z: 1,
            h_ratio: 1.0,
        }
    }

    pub fn from_idx2d(idx: IdxInRange2d, h_ratio: f32) -> UnitCellIdx {
        UnitCellIdx {
            x: idx.i.pct(),
            y: idx.j.pct(),
            z: 0.0,
            x_i: idx.i.i(),
            y_i: idx.j.i(),
            z_i: 0,
            seed: idx.to_seed() as f32,
            total_x: idx.i.total(),
            total_y: idx.j.total(),
            total_z: 1,
            h_ratio,
        }
    }

    pub fn from_idx3d(x_idx: IdxInRange, y_idx: IdxInRange, z_idx: IdxInRange) -> UnitCellIdx {
        let seed = z_idx.i() * (y_idx.total_usize() * x_idx.total_usize()) as u64
            + y_idx.i() * (x_idx.total_usize() as u64)
            + x_idx.i();

        UnitCellIdx {
            x: x_idx.pct(),
            y: y_idx.pct(),
            z: z_idx.pct(),
            x_i: x_idx.i(),
            y_i: y_idx.i(),
            z_i: z_idx.i(),
            seed: seed as f32,
            total_x: x_idx.total(),
            total_y: y_idx.total(),
            total_z: z_idx.total(),
            h_ratio: 1.0,
        }
    }

    pub fn to_idx2d(&self) -> IdxInRange2d {
        IdxInRange2d::new_from_idx(
            IdxInRange::new(self.x_i, self.total_x),
            IdxInRange::new(self.y_i, self.total_y),
        )
    }

    pub fn i(&self) -> u64 {
        self.x_i + self.y_i * self.total_x + self.z_i * (self.total_x * self.total_y)
    }

    pub fn max(&self) -> (u64, u64, u64) {
        (self.total_x, self.total_y, self.total_z)
    }

    pub fn seed(&self) -> f32 {
        self.seed
    }
}

impl IntoExprWorldContext for UnitCellIdx {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues {
        // make a few rns
        let mut rng = StdRng::seed_from_u64((self.seed + 19247.0) as u64);

        let rn0 = rng.gen_range(0.0..1.0);
        let rn1 = rng.gen_range(0.0..1.0);
        let rn2 = rng.gen_range(0.0..1.0);
        let rn3 = rng.gen_range(0.0..1.0);
        let rn4 = rng.gen_range(0.0..1.0);
        let rn5 = rng.gen_range(0.0..1.0);

        let v = vec![
            ("x".to_owned(), LivecodeValue::Float(self.x as f64)),
            ("y".to_owned(), LivecodeValue::Float(self.y as f64)),
            ("z".to_owned(), LivecodeValue::Float(self.z as f64)),
            ("x_i".to_owned(), LivecodeValue::Int(self.x_i as i64)),
            ("y_i".to_owned(), LivecodeValue::Int(self.y_i as i64)),
            ("z_i".to_owned(), LivecodeValue::Int(self.z_i as i64)),
            (
                "x_total".to_owned(),
                LivecodeValue::Int(self.total_x as i64),
            ),
            (
                "y_total".to_owned(),
                LivecodeValue::Int(self.total_y as i64),
            ),
            (
                "z_total".to_owned(),
                LivecodeValue::Int(self.total_z as i64),
            ),
            ("i".to_owned(), LivecodeValue::Int(self.i() as i64)),
            (
                "frac".to_owned(),
                LivecodeValue::Float(
                    self.i() as f64 / (self.total_x * self.total_y * self.total_z) as f64,
                ),
            ),
            (
                "total".to_owned(),
                LivecodeValue::Float((self.total_x * self.total_y * self.total_z) as f64),
            ),
            ("seed".to_owned(), LivecodeValue::Float(self.seed as f64)),
            ("rn0".to_owned(), LivecodeValue::Float(rn0)),
            ("rn1".to_owned(), LivecodeValue::Float(rn1)),
            ("rn2".to_owned(), LivecodeValue::Float(rn2)),
            ("rn3".to_owned(), LivecodeValue::Float(rn3)),
            ("rn4".to_owned(), LivecodeValue::Float(rn4)),
            ("rn5".to_owned(), LivecodeValue::Float(rn5)),
            (
                "h_ratio".to_owned(),
                LivecodeValue::Float(self.h_ratio as f64),
            ),
        ];
        ExprWorldContextValues::new(v)
    }
}

#[derive(Debug, Clone)]
pub enum UnitCellDetails {
    Wallpaper(UnitCellDetailsWallpaper),
    Function(UnitCellDetailsFunction), // this is a new one
}

impl UnitCellDetails {
    // for a while we just did wallpaper, so default to that
    pub fn new(transform_vertex: SimpleTransform2d) -> Self {
        Self::Wallpaper(UnitCellDetailsWallpaper {
            transform_vertex,
            adjust_shape: SimpleTransform2d::ident(),
            is_base: true,
        })
    }
    pub fn new_fancy(
        transform_vertex: SimpleTransform2d,
        adjust_shape: SimpleTransform2d,
        is_base: bool,
    ) -> Self {
        Self::Wallpaper(UnitCellDetailsWallpaper {
            transform_vertex,
            adjust_shape,
            is_base,
        })
    }

    pub fn new_func(func: Box<dyn Vec2TransformFunction>) -> UnitCellDetails {
        UnitCellDetails::Function(UnitCellDetailsFunction { func })
    }

    pub fn as_wallpaper(&self) -> Option<&UnitCellDetailsWallpaper> {
        if let Self::Wallpaper(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn transform(&self) -> SimpleTransform2d {
        match self {
            UnitCellDetails::Wallpaper(x) => x.transform(),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    fn transform_with_skew_mat4(&self) -> SimpleTransform2d {
        match self {
            UnitCellDetails::Wallpaper(x) => x.transform_with_skew_mat4(),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    pub fn transform_no_skew_one_point(&self, v: Vec2) -> Vec2 {
        self.transform_no_skew(&vec![v]).clone_to_vec()[0]
    }

    pub fn transform_no_skew<F: Transformable>(&self, v: &F) -> F {
        match self {
            UnitCellDetails::Wallpaper(w) => w.transform_no_skew(v),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    fn transform_no_skew_mat4(&self) -> SimpleTransform2d {
        match self {
            UnitCellDetails::Wallpaper(w) => w.transform_no_skew_mat(),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    pub fn adjust_shape(&self) -> SimpleTransform2d {
        match self {
            UnitCellDetails::Wallpaper(w) => w.adjust_shape(),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    fn is_base(&self) -> bool {
        match self {
            UnitCellDetails::Wallpaper(w) => w.is_base(),
            UnitCellDetails::Function(_) => true,
        }
    }

    pub(crate) fn experimental_lerp(&self, other: &UnitCellDetails, pct: f32) -> UnitCellDetails {
        match (self, other) {
            (UnitCellDetails::Wallpaper(w1), UnitCellDetails::Wallpaper(w2)) => {
                w1.experimental_lerp(w2, pct)
            }
            _ => {
                if pct > 0.5 {
                    self.clone()
                } else {
                    other.clone()
                }
            }
        }
    }
}

pub struct UnitCellDetailsFunction {
    func: Box<dyn Vec2TransformFunction>,
}

impl fmt::Debug for UnitCellDetailsFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UnitCellDetailsFunction {{ ... }}")
    }
}

impl Clone for UnitCellDetailsFunction {
    fn clone(&self) -> Self {
        UnitCellDetailsFunction {
            func: self.func.clone_box(),
        }
    }
}

impl UnitCellDetailsFunction {
    pub fn transform_with_skew(&self, x: Vec2) -> Vec2 {
        (self.func)(x)
    }
}

#[derive(Debug, Clone)]
pub struct UnitCellDetailsWallpaper {
    pub transform_vertex: SimpleTransform2d,
    pub adjust_shape: SimpleTransform2d,
    pub is_base: bool, // in cases of symmetry, will tell if this is the first one. useful for borders
}

impl UnitCellDetailsWallpaper {
    pub fn offset(&self) -> Vec2 {
        self.transform_with_skew(Vec2::ZERO)
    }

    pub fn transform_no_skew_mat(&self) -> SimpleTransform2d {
        // adjust the shape (symmetry, rotation), translate the center
        let offset = self.offset();
        let new_center = SimpleTransform2d::translate(offset);
        self.adjust_shape.add_transform_after(&new_center)
    }

    pub fn transform_no_skew<F: Transformable>(&self, v: &F) -> F {
        let m = self.transform_no_skew_mat();
        v.transform_with(&m)
    }

    // how to move the location of something
    pub fn transform_vertex(&self) -> SimpleTransform2d {
        self.transform_vertex.clone()
    }

    // how to transform a shape, e.g. rotation and flip
    pub fn adjust_shape(&self) -> SimpleTransform2d {
        self.adjust_shape.clone()
    }

    pub fn transform(&self) -> SimpleTransform2d {
        self.transform_vertex()
    }

    fn combine(&self, detail: &UnitCellDetails) -> UnitCellDetails {
        UnitCellDetails::Wallpaper(UnitCellDetailsWallpaper {
            transform_vertex: detail
                .as_wallpaper()
                .unwrap()
                .transform_vertex
                .add_transform_after(&self.transform_vertex),
            adjust_shape: detail
                .as_wallpaper()
                .unwrap()
                .adjust_shape
                .add_transform_after(&self.adjust_shape),
            is_base: self.is_base && detail.as_wallpaper().unwrap().is_base,
        })
    }

    fn transform_with_skew(&self, x: Vec2) -> Vec2 {
        self.transform_vertex.transform_vec2(x)
    }

    fn is_base(&self) -> bool {
        self.is_base
    }

    fn transform_with_skew_mat4(&self) -> SimpleTransform2d {
        self.transform_vertex.clone()
    }

    fn experimental_lerp(&self, other: &UnitCellDetailsWallpaper, pct: f32) -> UnitCellDetails {
        UnitCellDetails::Wallpaper(UnitCellDetailsWallpaper {
            transform_vertex: self.transform_vertex.lerpify(&other.transform_vertex, &pct),
            adjust_shape: self.adjust_shape.lerpify(&other.adjust_shape, &pct),
            is_base: self.is_base || other.is_base,
        })
    }
}
