use glam::*;
use murrelet_common::*;
use murrelet_draw::{
    draw::{CoreSDrawCtx, CoreSDrawCtxUnitCell, MurreletStyle, Sdraw},
    style::{MurreletPath, StyledPath},
};
use murrelet_livecode::unitcells::UnitCellContext;
use murrelet_perform::perform::SvgDrawConfig;
use murrelet_svg::svg::{SvgPathCache, SvgPathCacheRef};

// from the wasm-rust tutorial, this let's you log messages to the js console
// extern crate web_sys;
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

#[derive(Clone)]
pub struct WebSDrawCtxUnitCell {
    ctx: CoreSDrawCtxUnitCell,
    sdraw: WebSDrawCtx,
}

impl WebSDrawCtxUnitCell {
    pub fn draw_curve_path(&self, cd: MurreletPath) {
        // todo, this is a little clumsy
        let mut path = cd;
        path = path.transform_with(&self.ctx.unit_cell_transform());
        path = path.transform_with_mat4_after(self.sdraw.transform());

        self.sdraw.svg_draw.add_styled_path(
            "",
            StyledPath::new_from_path(path, self.svg_style().clone()),
        );
    }

    pub fn clear_context(&self) -> WebSDrawCtx {
        self.sdraw.clone()
    }

    pub fn with_unit_cell_skew(&self, skew: bool) -> Self {
        let mut c = self.clone();
        c.ctx = self.ctx.with_unit_cell_skew(skew);
        c
    }
}

impl Sdraw for WebSDrawCtxUnitCell {
    fn with_svg_style(&self, svg_style: murrelet_draw::draw::MurreletStyle) -> Self {
        let mut sdraw = self.clone();
        sdraw.ctx = self.ctx.with_svg_style(svg_style);
        sdraw
    }

    fn svg_style(&self) -> murrelet_draw::draw::MurreletStyle {
        self.ctx.svg_style()
    }

    fn transform(&self) -> Mat4 {
        self.ctx.transform()
    }

    fn set_transform(&self, m: Mat4) -> Self {
        let mut sdraw = self.clone();
        sdraw.ctx = self.ctx.set_transform(m);
        sdraw
    }

    fn transform_points<F: IsPolyline>(&self, face: &F) -> Polyline {
        self.ctx.transform_points(face)
    }

    fn line_space_multi(&self) -> f32 {
        self.ctx.line_space_multi()
    }
}

#[derive(Clone)]
pub struct WebSDrawCtx {
    ctx: CoreSDrawCtx,
    pub svg_draw: SvgPathCacheRef,
}

impl WebSDrawCtx {
    fn _draw_curve_path(&self, cd_raw: MurreletPath) {
        // todo, this is clunky
        let cd = cd_raw;

        let path = StyledPath::new_from_path(cd, self.svg_style().clone());

        self.svg_draw.add_styled_path("", path)
    }

    pub fn make_html(&self) -> Vec<String> {
        self.svg_draw.make_html()
    }

    pub fn add_guides(&self) {
        self.svg_draw.add_guides();
    }

    pub fn save_doc(&self) {
        self.svg_draw.save_doc();
    }

    pub fn with_detail(&self, detail: &UnitCellContext) -> WebSDrawCtxUnitCell {
        let ctx = self.ctx.with_detail(detail);
        WebSDrawCtxUnitCell {
            ctx,
            sdraw: self.clone(),
        }
    }

    pub fn new(svg_draw_config: &SvgDrawConfig) -> WebSDrawCtx {
        let svg_draw = SvgPathCache::svg_draw(svg_draw_config);

        let ctx = CoreSDrawCtx::new(
            MurreletStyle::new_white(false, false),
            svg_draw_config.frame() as f32,
            Mat4::IDENTITY,
        );

        WebSDrawCtx { svg_draw, ctx }
    }
}

impl Sdraw for WebSDrawCtx {
    fn with_svg_style(&self, svg_style: MurreletStyle) -> Self {
        let mut c = self.clone();
        c.ctx = c.ctx.with_svg_style(svg_style);
        c
    }

    fn svg_style(&self) -> MurreletStyle {
        self.ctx.svg_style()
    }

    fn transform(&self) -> Mat4 {
        self.ctx.transform()
    }

    fn set_transform(&self, m: Mat4) -> Self {
        let mut c = self.clone();
        c.ctx = c.ctx.set_transform(m);
        c
    }

    fn transform_points<F: IsPolyline>(&self, face: &F) -> Polyline {
        self.ctx.transform_points(face)
    }

    fn line_space_multi(&self) -> f32 {
        self.ctx.line_space_multi()
    }
}
