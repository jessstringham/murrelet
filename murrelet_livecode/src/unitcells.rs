use evalexpr::{build_operator_tree, HashMapContext, Node};
use glam::*;
use itertools::Itertools;
use murrelet_common::*;

use serde::Deserialize;
use std::fmt::Debug;
use std::{any::Any, collections::HashMap, fmt};

use crate::expr::{ExprWorldContextValues, IntoExprWorldContext};
use crate::livecode::{LivecodeError, LivecodeFromWorld, LivecodeResult};
use crate::state::LivecodeWorldState;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum UnitCellControlExprF32 {
    Int(i32),
    Bool(bool),
    Float(f32),
    Expr(Node),
}

impl UnitCellControlExprF32 {
    fn new(x: f32) -> UnitCellControlExprF32 {
        UnitCellControlExprF32::Float(x)
    }
}

impl EvaluableUnitCell<f32> for UnitCellControlExprF32 {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<f32> {
        match self {
            UnitCellControlExprF32::Bool(b) => {
                if *b {
                    Ok(1.0)
                } else {
                    Ok(-1.0)
                }
            }
            UnitCellControlExprF32::Int(i) => Ok(*i as f32),
            UnitCellControlExprF32::Float(x) => Ok(*x),
            UnitCellControlExprF32::Expr(e) => {
                match e.eval_float_with_context(ctx.ctx()).map(|x| x as f32) {
                    Ok(r) => Ok(r),
                    Err(_) => {
                        let b = e
                            .eval_boolean_with_context(ctx.ctx())
                            .map_err(|err| LivecodeError::EvalExpr(format!("evalexpr err"), err));
                        Ok(if b? { 1.0 } else { -1.0 })
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum UnitCellControlExprBool {
    Int(i32),
    Bool(bool),
    Float(f32),
    Expr(Node),
}

impl UnitCellControlExprBool {
    pub fn new(x: bool) -> UnitCellControlExprBool {
        UnitCellControlExprBool::Bool(x)
    }
}

impl EvaluableUnitCell<bool> for UnitCellControlExprBool {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<bool> {
        match self {
            UnitCellControlExprBool::Bool(b) => Ok(*b),
            UnitCellControlExprBool::Int(i) => Ok(*i > 0),
            UnitCellControlExprBool::Float(x) => Ok(*x > 0.0),
            UnitCellControlExprBool::Expr(e) => match e.eval_boolean_with_context(ctx.ctx()) {
                Ok(r) => Ok(r),
                Err(_) => {
                    let b = e.eval_float_with_context(ctx.ctx()).map_err(|err| {
                        LivecodeError::EvalExpr(format!("error evaluing bool"), err)
                    });
                    b.map(|x| x > 0.0)
                }
            },
        }
    }
}

// helps you translate between LiveCode and UnitCells
pub struct TmpUnitCells<CtxSource: UnitCellCreator, Target> {
    sequencer: CtxSource,
    node: Box<dyn EvaluableUnitCell<Target>>,
    ctx: Option<UnitCellCtx>,
    prefix: String,
}

impl<CtxSource: UnitCellCreator, Target: Default> TmpUnitCells<CtxSource, Target> {
    pub fn new(
        sequencer: CtxSource,
        node: Box<dyn EvaluableUnitCell<Target>>,
        ctx: Option<UnitCellCtx>,
        prefix: &str,
    ) -> Self {
        Self {
            sequencer,
            node,
            ctx,
            prefix: prefix.to_owned(),
        }
    }
}

fn create_unit_cell<'a>(
    world_ctx: &'a LivecodeWorldState,
    prefix: &'a str,
    unit_cell_ctx: &'a UnitCellContext,
    maybe_node: Option<&'a UnitCellCtx>,
) -> LivecodeResult<LivecodeWorldState> {
    // world_ctx is currently just the World, so first attach the unit cell world state

    let mut world_state = world_ctx.clone_to_unitcell(unit_cell_ctx, prefix)?;

    let mut unit_cell_world_ctx = world_state.ctx_mut();

    // now update the unit_cell context to have the node
    if let Some(node) = maybe_node {
        node.eval_raw(&mut unit_cell_world_ctx)?;
    }

    // great, now we have it built. return it!
    Ok(world_state)

    // let mut unit_cell_world_ctx = world_state.ctx().clone();

    // // now update the unit_cell context to have the node
    // if let Some(node) = maybe_node {
    //     node.eval_raw(&mut unit_cell_world_ctx)?;
    // }

    // // great, now we have it built. return it!
    // Ok(unit_cell_world_ctx)
}

impl<CtxSource, Target> TmpUnitCells<CtxSource, Target>
where
    CtxSource: UnitCellCreator,
    Target: Default + std::fmt::Debug + Clone,
{
    pub fn eval_with_ctx(
        &self,
        world_ctx: &LivecodeWorldState,
        unit_cell_ctx: &Option<UnitCellCtx>,
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
                    create_unit_cell(world_ctx, &self.prefix, ctx, unit_cell_ctx.as_ref());

                // and evaluate with this!
                // todo can i use the result to clean this up
                let node = match unit_cell_world_ctx_result {
                    Ok(unit_cell_world_ctx) => match self.node.eval(&unit_cell_world_ctx) {
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
        Ok(UnitCells::new(self.eval_with_ctx(&ctx, &self.ctx)))
    }
}

pub fn _auto_default_color_4_unitcell() -> [UnitCellControlExprF32; 4] {
    [
        UnitCellControlExprF32::new(0.0),
        UnitCellControlExprF32::new(0.0),
        UnitCellControlExprF32::new(0.0),
        UnitCellControlExprF32::new(0.0),
    ]
}

pub fn _auto_default_bool_false_unitcell() -> UnitCellControlExprF32 {
    UnitCellControlExprF32::new(-1.0)
}

pub fn _auto_default_bool_true_unitcell() -> UnitCellControlExprF32 {
    UnitCellControlExprF32::new(1.0)
}

pub fn _auto_default_vec3_0_unitcell() -> [UnitCellControlExprF32; 3] {
    [
        UnitCellControlExprF32::new(0.0),
        UnitCellControlExprF32::new(0.0),
        UnitCellControlExprF32::new(0.0),
    ]
}

pub fn _auto_default_vec2_0_unitcell() -> [UnitCellControlExprF32; 2] {
    [
        UnitCellControlExprF32::new(0.0),
        UnitCellControlExprF32::new(0.0),
    ]
}

pub fn _auto_default_vec2_1_unitcell() -> [UnitCellControlExprF32; 2] {
    [
        UnitCellControlExprF32::new(1.0),
        UnitCellControlExprF32::new(1.0),
    ]
}

pub fn _auto_default_0_unitcell() -> UnitCellControlExprF32 {
    UnitCellControlExprF32::new(0.0)
}

pub fn _auto_default_1_unitcell() -> UnitCellControlExprF32 {
    UnitCellControlExprF32::new(1.0)
}

/// for structs that can be used to generate a bunch of different contexts
/// e.g. Tiler, crystals
pub trait UnitCellCreator {
    fn to_unit_cell_ctxs(&self) -> Vec<UnitCellContext>;
}

/// this one's similar to LivecodeFromWorld, but for ones with unit_cell_context
pub trait EvaluableUnitCell<UnitCellTarget> {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<UnitCellTarget>;
}

impl EvaluableUnitCell<Vec2> for [UnitCellControlExprF32; 2] {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<Vec2> {
        Ok(vec2(self[0].eval(ctx)?, self[1].eval(ctx)?))
    }
}

impl EvaluableUnitCell<Vec3> for [UnitCellControlExprF32; 3] {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<Vec3> {
        Ok(vec3(
            self[0].eval(ctx)?,
            self[1].eval(ctx)?,
            self[2].eval(ctx)?,
        ))
    }
}

/// this one's similar to LivecodeToControl, but for unitcells
pub trait InvertedWorld<UnitCellControl> {
    fn to_unitcell_input(&self) -> UnitCellControl;
}

impl InvertedWorld<[UnitCellControlExprF32; 2]> for Vec2 {
    fn to_unitcell_input(&self) -> [UnitCellControlExprF32; 2] {
        [
            UnitCellControlExprF32::Float(self.x),
            UnitCellControlExprF32::Float(self.y),
        ]
    }
}

impl InvertedWorld<UnitCellControlExprF32> for f32 {
    fn to_unitcell_input(&self) -> UnitCellControlExprF32 {
        UnitCellControlExprF32::Float(*self)
    }
}

impl InvertedWorld<[UnitCellControlExprF32; 3]> for Vec3 {
    fn to_unitcell_input(&self) -> [UnitCellControlExprF32; 3] {
        [
            UnitCellControlExprF32::Float(self.x),
            UnitCellControlExprF32::Float(self.y),
            UnitCellControlExprF32::Float(self.z),
        ]
    }
}

impl InvertedWorld<UnitCellControlExprF32> for bool {
    fn to_unitcell_input(&self) -> UnitCellControlExprF32 {
        UnitCellControlExprF32::Bool(*self)
    }
}

impl InvertedWorld<UnitCellControlExprF32> for usize {
    fn to_unitcell_input(&self) -> UnitCellControlExprF32 {
        UnitCellControlExprF32::Int(*self as i32)
    }
}

impl InvertedWorld<UnitCellControlExprF32> for u64 {
    fn to_unitcell_input(&self) -> UnitCellControlExprF32 {
        UnitCellControlExprF32::Int(*self as i32)
    }
}

impl InvertedWorld<UnitCellControlExprF32> for u8 {
    fn to_unitcell_input(&self) -> UnitCellControlExprF32 {
        UnitCellControlExprF32::Int(*self as i32)
    }
}

impl InvertedWorld<UnitCellControlExprF32> for u32 {
    fn to_unitcell_input(&self) -> UnitCellControlExprF32 {
        UnitCellControlExprF32::Int(*self as i32)
    }
}

impl InvertedWorld<[UnitCellControlExprF32; 4]> for MurreletColor {
    fn to_unitcell_input(&self) -> [UnitCellControlExprF32; 4] {
        let [r, g, b, a] = self.into_rgba_components();
        [
            UnitCellControlExprF32::Float(r),
            UnitCellControlExprF32::Float(g),
            UnitCellControlExprF32::Float(b),
            UnitCellControlExprF32::Float(a),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct UnitCell<Target> {
    pub node: Box<Target>,
    pub detail: UnitCellContext,
}

impl<Target: Default> Default for UnitCell<Target> {
    fn default() -> Self {
        Self {
            node: Default::default(),
            detail: UnitCellContext::new(
                UnitCellExprWorldContext::from_idx1d(IdxInRange::new(0, 1)),
                Mat4::IDENTITY,
            ),
        }
    }
}

impl<Target> UnitCell<Target> {
    pub fn new(node: Target, detail: UnitCellContext) -> Self {
        Self {
            node: Box::new(node),
            detail,
        }
    }

    // can i deprecate it, it doesn't seem to move...
    // #[deprecated(info="just want to see where it's used")]
    pub fn transform_vec2(&self, v: Vec2) -> Vec2 {
        // let w = vec4(v.x, v.y, 0.0, 1.0);
        // let m = self.detail.transform() * w;
        // vec2(m.x / m.w, m.y / m.w)
        self.detail.transform().transform_vec2(v)
    }

    pub fn transform(&self) -> Mat4 {
        self.detail.transform()
    }

    // pub fn transform_offset_only(&self) -> Mat4 {
    //     self.detail.transform_offset_mat()
    // }

    // pub fn transform_offset_only(&self) -> Mat4 {
    //     self.detail.transform_offset_obj()
    // }

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
        self.detail.ctx.to_idx2d()
    }

    pub fn is_alternate(&self) -> bool {
        // !(self.idx().i.i() % 2 == 0) ^ (self.idx().j.i() % 2 == 0)
        self.idx().is_alternate()
    }
}

// this one is useful in sequencers
#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct UnitCellCtx(Node);

impl Default for UnitCellCtx {
    fn default() -> Self {
        Self(build_operator_tree("").unwrap())
    }
}

impl UnitCellCtx {
    pub fn eval_raw(&self, ctx: &mut HashMapContext) -> LivecodeResult<()> {
        self.0
            .eval_empty_with_context_mut(ctx)
            .map_err(|err| LivecodeError::EvalExpr("error evaluating ctx".to_owned(), err))
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct LazyNodeF32Def(Node);

impl LazyNodeF32Def {
    pub fn new(n: Node) -> Self {
        Self(n)
    }
}

impl LivecodeFromWorld<LazyNodeF32> for LazyNodeF32Def {
    fn o(&self, w: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(LazyNodeF32::new(self.0.clone(), w))
    }
}

impl EvaluableUnitCell<LazyNodeF32> for LazyNodeF32Def {
    fn eval(&self, ctx: &LivecodeWorldState) -> LivecodeResult<LazyNodeF32> {
        Ok(LazyNodeF32::new(self.0.clone(), ctx))
    }
}

// todo, hrm, this is awkward
#[derive(Debug, Clone)]
pub struct MixedEvalDefs {
    vals: ExprWorldContextValues,
    nodes: Vec<UnitCellCtx>, // these need to stack
}
impl MixedEvalDefs {
    pub fn new() -> Self {
        Self {
            vals: ExprWorldContextValues::new(vec![]),
            nodes: Vec::new(),
        }
    }

    pub fn set_panicle(&mut self, vals: ExprWorldContextValues) {
        self.vals = vals;
    }

    pub fn update_ctx(&self, ctx: &mut HashMapContext) -> LivecodeResult<()> {
        self.vals.update_ctx(ctx)?;
        // go from beginning to end
        for node in self.nodes.iter() {
            node.eval_raw(ctx)?;
        }

        Ok(())
    }

    pub fn set_val(&mut self, name: &str, val: LivecodeValue) {
        self.vals.set_val(name, val)
    }

    pub fn add_node(&mut self, node: UnitCellCtx) {
        self.nodes.push(node)
    }
}

// todo, figure out how to only build this context once per unitcell/etc

#[derive(Debug, Clone)]
pub struct LazyNodeF32Inner {
    n: Node,
    world: LivecodeWorldState,
}
impl LazyNodeF32Inner {
    pub fn eval_with_ctx(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<f32> {
        // start with the global ctx
        let mut ctx = self.world.clone();

        ctx.update_with_defs(more_defs)?;

        // modify it with the new data
        // todo, handle the result better
        more_defs.update_ctx(&mut ctx.ctx_mut())?;

        // now grab the actual node
        self.final_eval(&ctx.ctx())
    }

    pub fn final_eval(&self, ctx: &HashMapContext) -> LivecodeResult<f32> {
        self.n
            .eval_float_with_context(ctx)
            .map(|x| x as f32)
            .map_err(|err| LivecodeError::EvalExpr(format!("error evaluating lazy"), err))
    }

    pub fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<f32> {
        let pct = idx.pct();
        let i = idx.i();
        let total = idx.total();

        // todo, make this more standardized
        let vs: ExprWorldContextValues = ExprWorldContextValues::new(vec![
            ("_p".to_owned(), LivecodeValue::Float(pct as f64)),
            ("_i".to_owned(), LivecodeValue::Int(i as i64)),
            ("_total".to_owned(), LivecodeValue::Int(total as i64)),
        ]);

        let mut ctx = self.world.ctx().clone();

        vs.update_ctx_with_prefix(&mut ctx, prefix);

        self.final_eval(&ctx)
    }
}

// // expr that we can add things
#[derive(Debug, Clone)]
pub enum LazyNodeF32 {
    Uninitialized,
    Node(LazyNodeF32Inner),
}

impl Default for LazyNodeF32 {
    fn default() -> Self {
        LazyNodeF32::Uninitialized
    }
}

impl LazyNodeF32 {
    pub fn new(n: Node, world: &LivecodeWorldState) -> Self {
        Self::Node(LazyNodeF32Inner {
            n,
            world: world.clone_to_lazy(),
        })
    }

    // fn new_from_livecode(n: Node, w: &LiveCodeWorldState) -> Self {
    //     Self::Node(LazyNodeF32Inner {
    //         n,
    //         world: LivecodeWorldState::from_world(w).into_lazy(),
    //     })
    // }

    pub fn n(&self) -> Option<&Node> {
        match self {
            LazyNodeF32::Uninitialized => None,
            LazyNodeF32::Node(n) => Some(&n.n),
        }
    }

    pub fn eval_with_ctx(&self, more_defs: &MixedEvalDefs) -> LivecodeResult<f32> {
        self.node()?.eval_with_ctx(more_defs)
    }

    pub fn final_eval(&self, ctx: &HashMapContext) -> LivecodeResult<f32> {
        self.node()?.final_eval(ctx)
    }

    pub fn eval_idx(&self, idx: IdxInRange, prefix: &str) -> LivecodeResult<f32> {
        self.node()?.eval_idx(idx, prefix)
    }

    pub fn node(&self) -> LivecodeResult<&LazyNodeF32Inner> {
        if let Self::Node(v) = self {
            Ok(v)
        } else {
            Err(LivecodeError::Raw(
                "trying to use uninitialized lazy node".to_owned(),
            ))
        }
    }
}

impl<'a> InvertedWorld<LazyNodeF32Def> for LazyNodeF32 {
    fn to_unitcell_input(&self) -> LazyNodeF32Def {
        LazyNodeF32Def(self.n().unwrap().clone())
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

    pub fn iter(&self) -> std::slice::Iter<UnitCell<Target>> {
        self.items.iter()
    }

    pub fn x_y_z_max(&self) -> (u64, u64, u64) {
        // each should have the same, so grab the first
        if let Some(first) = self.items.first() {
            first.detail.ctx.max()
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
                    item.detail.ctx.x_i,
                    item.detail.ctx.y_i,
                    item.detail.ctx.z_i,
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
}

impl<Target: std::fmt::Debug + Clone + Default> FromIterator<UnitCell<Target>>
    for UnitCells<Target>
{
    fn from_iter<I: IntoIterator<Item = UnitCell<Target>>>(iter: I) -> Self {
        let vec: Vec<_> = iter.into_iter().collect();
        UnitCells { items: vec }
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
                let jj = if i % 2 == 0 { j + 1 } else { j };
                self.get_ij(i - 1, jj)
            }
            CellNeighbor::Hex(HexCellNeighbor::DownLeft) => {
                let jj = if i % 2 == 0 { j } else { j - 1 };
                self.get_ij(i - 1, jj)
            }
            CellNeighbor::Hex(HexCellNeighbor::Down) => self.get_ij(i, j - 1),
            CellNeighbor::Hex(HexCellNeighbor::DownRight) => {
                let jj = if i % 2 == 0 { j } else { j - 1 };
                self.get_ij(i + 1, jj)
            }
            CellNeighbor::Hex(HexCellNeighbor::UpRight) => {
                let jj = if i % 2 == 0 { j + 1 } else { j };
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

pub trait TileInfo: Debug + Any {
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
    ctx: UnitCellExprWorldContext,
    pub detail: UnitCellDetails,
    pub tile_info: Option<Box<dyn TileInfo>>,
}
impl UnitCellContext {
    pub fn new(ctx: UnitCellExprWorldContext, transform: Mat4) -> UnitCellContext {
        UnitCellContext {
            ctx,
            detail: UnitCellDetails::new(transform),
            tile_info: None,
        }
    }

    pub fn new_with_base(
        ctx: UnitCellExprWorldContext,
        detail: UnitCellDetails,
    ) -> UnitCellContext {
        UnitCellContext {
            ctx,
            detail,
            tile_info: None,
        }
    }

    pub fn set_tile_info(&mut self, tile_info: Box<dyn TileInfo>) {
        self.tile_info = Some(tile_info);
        // Self {
        //     tile_info: Some(tile_info),
        //     ..self.clone()
        // }
    }

    pub fn new_with_info(
        ctx: UnitCellExprWorldContext,
        detail: UnitCellDetails,
        tile_info: Box<dyn TileInfo>,
    ) -> UnitCellContext {
        UnitCellContext {
            ctx,
            detail,
            tile_info: Some(tile_info),
        }
    }

    // pub fn face(&self) -> Option<Vec<Vec2>> {
    //     // todo, replace this with rect_bound logic?
    //     self.tile_info.as_ref().map(|x| self.detail.transform_obj(x.face()))
    // }

    pub fn rect_for_face(&self) -> Rect {
        // todo, replace this with rect_bound
        let mut b = BoundMetric::new();
        b.add_points(&self.rect_bound());
        b.as_rect()
    }

    // just updates details...
    // applies the other transform _after_ current
    pub fn combine(&self, other: &UnitCellContext) -> UnitCellContext {
        UnitCellContext {
            ctx: self.ctx,
            detail: other.detail.as_wallpaper().unwrap().combine(&self.detail),
            tile_info: None,
        }
    }

    pub fn combine_keep_other_ctx(&self, other: &UnitCellContext) -> UnitCellContext {
        UnitCellContext {
            ctx: other.ctx,
            detail: other.detail.as_wallpaper().unwrap().combine(&self.detail),
            tile_info: None,
        }
    }

    pub fn ctx(&self) -> UnitCellExprWorldContext {
        self.ctx
    }

    pub fn transform(&self) -> Mat4 {
        self.detail.transform()
    }

    pub fn idx(&self) -> IdxInRange2d {
        self.ctx.to_idx2d()
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
        // todo, add in tektonics?
        self.transform_with_skew(&face).clone_to_vec()
    }

    pub fn transform_with_skew_mat4(&self) -> Mat4 {
        self.detail.transform_with_skew_mat4()
    }

    pub fn transform_with_skew<F: IsPolyline>(&self, v: &F) -> Polyline {
        self.detail.transform_with_skew(v)
    }

    pub fn transform_one_point_with_skew(&self, v: Vec2) -> Vec2 {
        self.detail.transform_with_skew(&vec![v]).clone_to_vec()[0]
    }

    pub fn is_base(&self) -> bool {
        self.detail.is_base()
    }

    // pub fn transform_offset_mat(&self) -> Mat4 {
    //     self.detail.transform_offset_mat()
    // }

    // pub fn transform_offset_only(&self, v: &Vec<Vec2>) -> Vec<Vec2> {
    //     self.transform_no_skew(v)
    // }

    pub fn adjust_shape(&self) -> Mat4 {
        self.detail.adjust_shape()
    }

    pub fn transform_no_skew_one_point(&self, v: Vec2) -> Vec2 {
        // also does adjust shape..
        self.detail.transform_no_skew_one_point(v)
    }

    pub fn transform_no_skew<F: IsPolyline>(&self, v: &F) -> Polyline {
        // also does adjust shape..
        self.detail.transform_no_skew(v)
    }

    pub fn transform_no_skew_mat4(&self) -> Mat4 {
        self.detail.transform_no_skew_mat4()
    }
}

impl IntoExprWorldContext for UnitCellContext {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues {
        self.ctx.as_expr_world_context_values()
    }
}

// world state for unit cell
#[derive(Copy, Clone, Debug)]
pub struct UnitCellExprWorldContext {
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
impl UnitCellExprWorldContext {
    pub fn from_idx2d_and_actual_xy(
        xy: Vec2,
        idx: IdxInRange2d,
        h_ratio: f32,
    ) -> UnitCellExprWorldContext {
        UnitCellExprWorldContext {
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

    pub fn from_idx1d(idx: IdxInRange) -> UnitCellExprWorldContext {
        UnitCellExprWorldContext {
            x: idx.pct() + idx.half_step_pct(),
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

    pub fn from_idx2d(idx: IdxInRange2d, h_ratio: f32) -> UnitCellExprWorldContext {
        UnitCellExprWorldContext {
            x: idx.i.pct() + idx.i.half_step_pct(),
            y: idx.j.pct() + idx.j.half_step_pct(),
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

    pub fn from_idx3d(
        x_idx: IdxInRange,
        y_idx: IdxInRange,
        z_idx: IdxInRange,
    ) -> UnitCellExprWorldContext {
        let seed = z_idx.i() * (y_idx.total_usize() * x_idx.total_usize()) as u64
            + y_idx.i() * (x_idx.total_usize() as u64)
            + x_idx.i();

        UnitCellExprWorldContext {
            x: x_idx.pct() + x_idx.half_step_pct(),
            y: y_idx.pct() + y_idx.half_step_pct(),
            z: z_idx.pct() + z_idx.half_step_pct(),
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

impl IntoExprWorldContext for UnitCellExprWorldContext {
    fn as_expr_world_context_values(&self) -> ExprWorldContextValues {
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
    pub fn new(transform_vertex: Mat4) -> Self {
        Self::Wallpaper(UnitCellDetailsWallpaper {
            transform_vertex,
            adjust_shape: Mat4::IDENTITY,
            is_base: true,
        })
    }
    pub fn new_fancy(transform_vertex: Mat4, adjust_shape: Mat4, is_base: bool) -> Self {
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

    fn transform(&self) -> Mat4 {
        match self {
            UnitCellDetails::Wallpaper(x) => x.transform(),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    fn transform_with_skew_mat4(&self) -> Mat4 {
        match self {
            UnitCellDetails::Wallpaper(x) => x.transform_with_skew_mat4(),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    fn transform_with_skew<F: IsPolyline>(&self, face: &F) -> Polyline {
        let vs = face
            .into_iter_vec2()
            .map(|x| match self {
                UnitCellDetails::Wallpaper(d) => d.transform_with_skew(x),
                UnitCellDetails::Function(d) => d.transform_with_skew(x),
            })
            .collect_vec();
        Polyline::new(vs)
    }

    // fn transform_offset_mat(&self) -> Mat4 {
    //     match self {
    //         UnitCellDetails::Wallpaper(w) => w.transform_no_skew_mat(),
    //         UnitCellDetails::Function(_) => todo!(),
    //     }
    // }

    pub fn transform_no_skew_one_point(&self, v: Vec2) -> Vec2 {
        self.transform_no_skew(&vec![v.clone()]).clone_to_vec()[0]
    }

    pub fn transform_no_skew<F: IsPolyline>(&self, v: &F) -> Polyline {
        match self {
            UnitCellDetails::Wallpaper(w) => w.transform_no_skew(v),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    fn transform_no_skew_mat4(&self) -> Mat4 {
        match self {
            UnitCellDetails::Wallpaper(w) => w.transform_no_skew_mat(),
            UnitCellDetails::Function(_) => todo!(),
        }
    }

    pub fn adjust_shape(&self) -> Mat4 {
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
    fn transform_with_skew(&self, x: Vec2) -> Vec2 {
        (self.func)(x)
    }
}

#[derive(Debug, Clone)]
pub struct UnitCellDetailsWallpaper {
    pub transform_vertex: Mat4,
    pub adjust_shape: Mat4,
    pub is_base: bool, // in cases of symmetry, will tell if this is the first one. useful for borders
}

impl UnitCellDetailsWallpaper {
    pub fn offset(&self) -> Vec2 {
        self.transform_with_skew(Vec2::ZERO)
    }

    pub fn transform_no_skew_mat(&self) -> Mat4 {
        // adjust the shape (symmetry, rotation), translate the center
        let new_center = Mat4::from_vec2_translate(self.offset());
        new_center * self.adjust_shape
    }

    pub fn transform_no_skew<F: IsPolyline>(&self, v: &F) -> Polyline {
        let m = self.transform_no_skew_mat();
        Polyline::new(
            v.into_iter_vec2()
                .map(|x| m.transform_vec2(x))
                .collect_vec(),
        )
    }

    // how to move the location of something
    pub fn transform_vertex(&self) -> Mat4 {
        self.transform_vertex
    }

    // how to transform a shape, e.g. rotation and flip
    pub fn adjust_shape(&self) -> Mat4 {
        self.adjust_shape
    }

    pub fn transform(&self) -> Mat4 {
        self.transform_vertex()
    }

    fn combine(&self, detail: &UnitCellDetails) -> UnitCellDetails {
        UnitCellDetails::Wallpaper(UnitCellDetailsWallpaper {
            transform_vertex: detail.as_wallpaper().unwrap().transform_vertex
                * self.transform_vertex,
            adjust_shape: detail.as_wallpaper().unwrap().adjust_shape * self.adjust_shape,
            is_base: self.is_base && detail.as_wallpaper().unwrap().is_base,
        })
    }

    fn transform_with_skew(&self, x: Vec2) -> Vec2 {
        self.transform_vertex.transform_vec2(x)
    }

    fn is_base(&self) -> bool {
        self.is_base
    }

    fn transform_with_skew_mat4(&self) -> Mat4 {
        self.transform_vertex
    }
}
