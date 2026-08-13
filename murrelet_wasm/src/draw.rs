use glam::*;
use murrelet_common::*;
use murrelet_draw::{
    draw::{CoreSDrawCtx, MurreletStyle, Sdraw},
    drawable::{DrawTarget, MixedDrawableShape, ToMixedDrawables},
    style::{MurreletPath, StyledPath},
};
use murrelet_perform::perform::SvgDrawConfig;
use murrelet_svg::svg::{StyledText, SvgPathCache, SvgPathCacheRef};
use regex::Regex;
use wasm_bindgen::JsValue;

// from the wasm-rust tutorial, this let's you log messages to the js console
// extern crate web_sys;
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

// chatgpt
pub fn round_decimals_in_text(s: &str, decimals: usize) -> String {
    let pattern = format!(r"(-?\d*\.\d{{{},}})", decimals);
    let re = Regex::new(&pattern).unwrap();

    re.replace_all(s, |caps: &regex::Captures| {
        let val: f64 = caps[0].parse().unwrap_or(0.0);
        format!("{:.decimals$}", val, decimals = decimals)
    })
    .into_owned()
}

#[derive(Clone)]
pub struct WebSDrawCtx {
    ctx: CoreSDrawCtx,
    pub svg_draw: SvgPathCacheRef,
}

impl WebSDrawCtx {
    pub fn draw_curve_path(&self, cd: MurreletPath) {
        let transformed_path = cd.transform_with(&self.transform());

        self.svg_draw.add_styled_path(
            "",
            StyledPath::new_from_path(transformed_path, self.svg_style()),
        );
    }

    pub fn draw_curve_path_with_annotation(&self, cd: MurreletPath, annotation: (String, String)) {
        self.svg_draw.add_styled_path(
            "",
            StyledPath::new_from_path_with_annotation(cd, self.svg_style(), annotation),
        );
    }

    pub fn draw_text(&self, s: String, v: Vec2) {
        self.svg_draw
            .add_styled_text("", StyledText::new(s, v, 180.0, self.svg_style()));
    }

    pub fn new_from_path_with_multiple_annotations(&self, cd: MurreletPath, annotations: Vec<(String, String)>) {
        self.svg_draw.add_styled_path(
            "",
            StyledPath::new_from_path_with_multiple_annotations(cd, self.svg_style(), annotations),
        );
    }

    pub fn drawn_shapes<D>(&self, v: &D)
    where
        D: ToMixedDrawables,
    {
        // The interactive web view is on-screen-like → DrawTarget::Screen.
        // (Keeping this Screen matches today's behavior exactly; the default
        // to_mixed_drawables_for delegates to to_mixed_drawables regardless.)
        for shape in v.to_mixed_drawables_for(DrawTarget::Screen) {
            let style = shape.style();
            match shape {
                MixedDrawableShape::Shape(shape) => {
                    let annotations = shape.annotations();
                    let ctx = self.with_svg_style(style.to_style());
                    for cd in shape.curves() {
                        let path = MurreletPath::curve(cd.clone()).transform_with_mat4_after(self.transform());
                        if annotations.is_empty() {
                            ctx.draw_curve_path(path);
                        } else {
                            ctx.svg_draw.add_styled_path(
                                "",
                                StyledPath::new_from_path_with_multiple_annotations(
                                    path,
                                    ctx.svg_style(),
                                    annotations.vals().clone(),
                                ),
                            );
                        }
                    }
                }
                MixedDrawableShape::Text(text) => {
                    for t in text.positions() {
                        self.with_svg_style(style.to_style()).draw_text(t.text().to_string(), t.loc());
                    }
                }
            }
        }
    }

    pub fn make_html(&self) -> (String, String) {
        self.svg_draw.make_html()
    }

    // (defs, one markup fragment per shape) for node-render mode.
    pub fn make_html_fragments(&self) -> (String, Vec<String>) {
        self.svg_draw.make_html_fragments()
    }

    pub fn make_html_jsvalue(&self, maybe_decimals: Option<usize>) -> JsValue {
        let (defs, paths) = self.svg_draw.make_html();

        // hmm, figure out a better way to do this, but quantize it

        let path_str = if let Some(decimal) = maybe_decimals {
            round_decimals_in_text(&paths, decimal)
        } else {
            paths
        };

        JsValue::from_str(&format!("{}\n{}", defs, path_str))
    }

    pub fn add_guides(&self) {
        self.svg_draw.add_guides();
    }

    pub fn save_doc(&self) {
        self.svg_draw.save_doc();
    }

    pub fn new(svg_draw_config: &SvgDrawConfig) -> WebSDrawCtx {
        let svg_draw = SvgPathCache::svg_draw(svg_draw_config);

        let ctx = CoreSDrawCtx::new(
            MurreletStyle::new_white(false, false),
            svg_draw_config.frame() as f32,
            SimpleTransform2d::ident(),
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

    fn transform(&self) -> SimpleTransform2d {
        self.ctx.transform()
    }

    fn set_transform<M: ToSimpleTransform>(&self, m: &M) -> Self {
        let mut sdraw = self.clone();
        sdraw.ctx = self.ctx.set_transform(m);
        sdraw
    }

    fn transform_points<F: Transformable>(&self, face: &F) -> F {
        self.ctx.transform_points(face)
    }

    fn line_space_multi(&self) -> f32 {
        self.ctx.line_space_multi()
    }
}
