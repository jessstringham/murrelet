pub mod draw;

use draw::{WebSDrawCtx, WebSDrawCtxUnitCell};
use glam::*;

use murrelet::prelude::*;
use murrelet_common::mat4_from_mat3_transform;
use murrelet_draw::{compass::*, draw::*, sequencers::*, style::styleconf::*};
use murrelet_svg::svg::MurreletPath;
use wasm_bindgen::prelude::*;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// from the wasm-rust tutorial, this let's you log messages to the js console
// extern crate web_sys;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into())
//     }
// }

#[derive(Debug, Clone, UnitCell, Default, Boop)]
struct StyledShape {
    shape: MurreletCompass,
    style: StyleConf,
    #[livecode(serde_default = "false")]
    skew: bool,
}
impl StyledShape {
    fn draw(&self, draw_ctx: &WebSDrawCtxUnitCell) {
        // first do the simple transform

        draw_ctx
            .with_unit_cell_skew(self.skew)
            .with_style(self.style.clone())
            .draw_curve_path(MurreletPath::curve(self.shape.to_curve_maker()));
    }
}

#[derive(Debug, Clone, UnitCell, Default, Boop)]
struct SimpleTile(Vec<StyledShape>);
impl SimpleTile {
    fn draw(&self, draw_ctx: &WebSDrawCtxUnitCell) {
        for v in &self.0 {
            v.draw(draw_ctx);
        }
    }
}

#[derive(Debug, Clone, Livecode)]
struct DrawingConfig {
    #[livecode(serde_default = "false")]
    debug: bool,
    sequencer: Sequencer,
    ctx: UnitCellCtx,
    #[livecode(src = "sequencer", ctx = "ctx")]
    node: UnitCells<SimpleTile>,
    offset: Vec2,
    scale: f32,
}
impl DrawingConfig {
    fn draw(&self, draw_ctx: &WebSDrawCtx) {
        // todo, this is a little clunky.. and isn't so clunky in my other code hrm
        let transform = mat4_from_mat3_transform(
            Mat3::from_scale(Vec2::ONE * self.scale) * Mat3::from_translation(self.offset),
        );

        for t in self.node.iter() {
            t.node
                .draw(&draw_ctx.set_transform(transform).with_detail(&t.detail))
        }
    }
}

// set up livecoder
#[derive(Debug, Clone, Livecode, TopLevelLiveCode)]
struct LiveCodeConf {
    app: AppConfig,
    drawing: DrawingConfig,
}

#[wasm_bindgen]
pub async fn new_model(conf: String) -> WasmMurreletModelResult {
    MurreletModel::new(conf).await
}

#[wasm_bindgen]
pub struct MurreletModel {
    livecode: LiveCode,
}
#[wasm_bindgen]
impl MurreletModel {
    #[wasm_bindgen(constructor)]
    pub async fn new(conf: String) -> WasmMurreletModelResult {
        // turn this on if you need to debug
        // std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        let livecode_src = LivecodeSrc::new(vec![Box::new(AppInputValues::new(false))]);

        match LiveCode::new_web(conf, livecode_src) {
            Ok(livecode) => {
                let r = MurreletModel { livecode };
                WasmMurreletModelResult::ok(r)
            }
            Err(e) => WasmMurreletModelResult::err(e),
        }
    }

    #[wasm_bindgen]
    pub fn update_config(&mut self, conf: String) -> String {
        match self.livecode.update_config_to(&conf) {
            Ok(_) => "Success!".to_owned(),
            Err(e) => e,
        }
    }

    #[wasm_bindgen]
    pub fn update_frame(
        &mut self,
        frame: u64,
        dim_x: f32,
        dim_y: f32,
        mouse_x: f32,
        mouse_y: f32,
        click: bool,
    ) {
        let app_input =
            MurreletAppInput::new_no_key(vec2(dim_x, dim_y), vec2(mouse_x, mouse_y), click, frame);
        self.livecode.update(&app_input, false);
    }

    // useful if you have shaders, this will list what canvases to draw to
    #[wasm_bindgen]
    pub fn canvas_ids(&self) -> Vec<String> {
        Vec::new()
    }

    // useful if you have shaders, this should generate the DOM for image textures in the svg
    #[wasm_bindgen]
    pub fn make_img_defs(&self) -> String {
        String::new()
    }

    #[wasm_bindgen]
    pub fn draw(&self) -> Vec<String> {
        let svg_draw_config = self.livecode.svg_save_path();
        let draw_ctx = WebSDrawCtx::new(&svg_draw_config);

        self.livecode.config().drawing.draw(&draw_ctx);

        draw_ctx.make_html()
    }

    #[wasm_bindgen]
    pub fn fps(&self) -> f32 {
        self.livecode.app_config().time.fps
    }

    #[wasm_bindgen]
    pub fn bg_color(&self) -> String {
        self.livecode.app_config().bg_color.to_svg_rgb()
    }
}

// just creating a Result<Model, String> that we can send to javascript
#[wasm_bindgen]
pub struct WasmMurreletModelResult {
    m: Option<MurreletModel>,
    err: String,
}

#[wasm_bindgen]
impl WasmMurreletModelResult {
    fn ok(m: MurreletModel) -> WasmMurreletModelResult {
        WasmMurreletModelResult {
            m: Some(m),
            err: String::new(),
        }
    }

    fn err(err: String) -> WasmMurreletModelResult {
        WasmMurreletModelResult { m: None, err }
    }

    #[wasm_bindgen]
    pub fn is_err(&self) -> bool {
        self.m.is_none()
    }

    #[wasm_bindgen]
    pub fn err_msg(self) -> String {
        self.err
    }

    #[wasm_bindgen]
    pub fn to_model(self) -> MurreletModel {
        // panics if you don't check is error first
        self.m.unwrap()
    }
}
