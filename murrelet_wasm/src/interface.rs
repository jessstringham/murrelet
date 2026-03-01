use std::fmt::Display;

use glam::vec2;
use murrelet_common::MurreletAppInput;
use murrelet_draw::drawable::MixedDrawableShape;
use murrelet_gen::embedding::MurreletQuantizedEmbedding;
use murrelet_perform::AppConfig;
use serde::Deserialize;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

pub type JsResult<T> = Result<T, JsValue>;

pub trait ToJsResult<T> {
    fn to_js(self) -> Result<T, JsValue>;
}

impl<T, E: Display> ToJsResult<T> for Result<T, E> {
    #[inline]
    fn to_js(self) -> Result<T, JsValue> {
        self.map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

pub trait IsMurreletWebModel<Conf, DrawOpts>
where
    for<'de> DrawOpts: Deserialize<'de>,
    Conf: Clone,
{
    fn init(conf: Conf) -> Self;
    fn get_conf(&self) -> &Conf;
    fn set_conf(&mut self, conf: Conf);

    fn reload(&mut self);

    fn update(&mut self, app_input: &MurreletAppInput);
    fn draw(&self, conf: &DrawOpts) -> Vec<MixedDrawableShape>;
}

// just so we can manage these things outside of a macro...
pub struct AppManager {
    pub conf: AppConfig,
    pub state: MurreletAppInput,
}
impl AppManager {
    pub fn new() -> Self {
        Self {
            conf: AppConfig::default_web(),
            state: MurreletAppInput::default_with_frames(1),
        }
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.conf.set_bpm(bpm);
    }

    pub fn set_beats_per_bar(&mut self, bpm: f32) {
        self.conf.set_beats_per_bar(bpm);
    }

    pub fn set_custom_var(&mut self, key: String, value: f32) {
        self.state.custom_vars.insert(key, value);
    }

    pub fn set_window_dims(&mut self, x: f32, y: f32) {
        self.state.window_dims = vec2(x, y);
    }

    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.state.mouse_position = vec2(x, y);
    }

    pub fn set_mouse_left_is_down(&mut self) {
        self.state.mouse_left_is_down = true;
    }

    pub fn set_mouse_left_is_up(&mut self) {
        self.state.mouse_left_is_down = false;
    }

    pub fn tick(&mut self) -> &murrelet_common::MurreletAppInput {
        self.state.elapsed_frames += 1;
        self.state()
    }

    pub fn conf(&self) -> &murrelet_perform::AppConfig {
        &self.conf
    }

    pub fn state(&self) -> &murrelet_common::MurreletAppInput {
        &self.state
    }
}

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, SvgElement, SvgsvgElement, window};

pub struct MurreletSvgObj {
    svg_el: SvgsvgElement,
    paths_g_el: SvgElement,
}

impl MurreletSvgObj {
    pub fn set_paths_inner_html(&self, markup: &str) {
        let el: &Element = self.paths_g_el.as_ref();
        el.set_inner_html(markup);
    }

    pub fn clear_paths(&self) {
        let el: &Element = self.paths_g_el.as_ref();
        el.set_inner_html("");
    }

    pub fn svg_outer_html(&self) -> String {
        self.svg_el.outer_html()
    }
}

// more chatgpt translation from my js to rust
pub fn new_svg_obj() -> Result<MurreletSvgObj, wasm_bindgen::JsValue> {
    let document: Document = window().ok_or("no window")?.document().ok_or("no document")?;

    let svg_ns = Some("http://www.w3.org/2000/svg");

    let svg_el = document.create_element_ns(svg_ns, "svg")?.dyn_into::<SvgsvgElement>()?;
    svg_el.set_attribute("viewBox", "0 0 800 800")?;

    let paths_g_el = document.create_element_ns(svg_ns, "g")?.dyn_into::<SvgElement>()?;
    paths_g_el.set_id("paths");

    svg_el.append_child(&paths_g_el)?;

    Ok(MurreletSvgObj { svg_el, paths_g_el })
}

#[macro_export]
macro_rules! export_murrelet_web_model {
    ($model_ty:ty, $conf_ty:ty, $draw_opts_ty:ty) => {
        paste::paste! {
            use murrelet_perform::perform::{ControlAppConfig, ControlLazyAppConfig};
            use murrelet_gen::CanSampleFromDist;
            use wasm_bindgen::prelude::*;
            use murrelet_livecode::livecode::LivecodeToControl;
            use web_sys::{Document, Element, SvgElement, SvgsvgElement, Window};
            use murrelet_gui::CanMakeGUI;


            #[wasm_bindgen]
            pub struct [<$conf_ty Wrapper>]($conf_ty);

            impl LivecodeToControl<[<Control $conf_ty>]> for [<$conf_ty Wrapper>] {
                fn to_control(&self) -> [<Control $conf_ty>] {
                    self.0.to_control()
                }
            }

            #[derive(Debug, Clone, Livecode, MurreletGUI, Lerpable, TopLevelLiveCodeJson)]
            pub struct [<$conf_ty TopLevelLivecode>] {
                pub drawing: $conf_ty,
                pub app: murrelet_perform::AppConfig,
            }
            impl [<$conf_ty TopLevelLivecode>] {
                fn new(model: &$model_ty, app_mng: &murrelet_wasm::interface::AppManager) -> Self {
                    Self {
                        drawing: model.get_conf().clone(),
                        app: app_mng.conf().clone(),
                    }
                }
            }

            impl WithDrawerUpdator<[<Control $conf_ty>]> for [<Control $conf_ty TopLevelLivecode>] {
                fn new_from_parts(app: murrelet_perform::ControlAppConfig, drawing: [<Control $conf_ty>]) -> Self {
                    [<Control $conf_ty TopLevelLivecode>] {
                        app, drawing
                    }
                }

                fn parse_drawer(text: &str) -> murrelet_livecode::types::LivecodeResult<[<Control $conf_ty>]> {
                    serde_json::from_str(&text).map_err(|err| {
                        murrelet_livecode::types::LivecodeError::JsonParse(err.to_string())
                    })
                }

                fn set_drawing_conf(&mut self, drawing_conf: [<Control $conf_ty>]) {
                    self.drawing = drawing_conf;
                }

                fn set_app(&mut self, app: murrelet_perform::ControlAppConfig) {
                    self.app = app;
                }
            }


            #[wasm_bindgen]
            pub struct [<$model_ty TopLevelWasm>] {
                livecode: LiveCode,
                model: $model_ty,
                app_mng: murrelet_wasm::interface::AppManager,
                svg_obj: Option<MurreletSvgObj>,
            }

            impl [<$model_ty TopLevelWasm>] {
                pub fn new_conf_livecode(conf: &str) -> murrelet_livecode::types::LivecodeResult<Self> {
                    let app_mng = murrelet_wasm::interface::AppManager::new();

                    let control_config = [<Control $conf_ty TopLevelLivecode>]::from_json(&app_mng.conf, conf)?;
                    let livecode = LiveCode::new_wasm(control_config)?;

                    let model = $model_ty::init(livecode.config().drawing.clone());
                    let rn_count = $conf_ty::rn_count();

                    Ok(Self {
                        model,
                        app_mng,
                        livecode,
                        svg_obj: None,
                    })
                }

                pub fn new_livecode(conf: &$conf_ty) -> murrelet_livecode::types::LivecodeResult<Self> {
                    let app_mng = murrelet_wasm::interface::AppManager::new();

                    let control_config = [<Control $conf_ty TopLevelLivecode>]::from_regular(&app_mng.conf, conf)?;
                    let livecode = LiveCode::new_wasm(control_config)?;

                    let model = $model_ty::init(livecode.config().drawing.clone());

                    let rn_count = $conf_ty::rn_count();


                    Ok(Self {
                        model,
                        app_mng,
                        livecode,
                        svg_obj: None,
                    })
                }

                pub fn set_config_json_internal(&mut self, drawer_str: &str) -> murrelet_livecode::types::LivecodeResult<()> {
                    let control_config = [<Control $conf_ty TopLevelLivecode>]::from_json(&self.app_mng.conf, drawer_str)?;

                    self.livecode.update_config_directly(control_config)?;

                    Ok(())
                }

                pub fn set_config_internal(&mut self, drawer_str: &[<$conf_ty Wrapper>]) -> murrelet_livecode::types::LivecodeResult<()> {
                    let control_config = [<Control $conf_ty TopLevelLivecode>]::from_regular(&self.app_mng.conf, drawer_str)?;

                    self.livecode.update_config_directly(control_config)?;

                    Ok(())
                }

                pub fn parse_draw_opts(conf: &str) -> murrelet_livecode::types::LivecodeResult<$draw_opts_ty> {
                    serde_json::from_str(&conf).map_err(|err| {
                        murrelet_livecode::types::LivecodeError::JsonParse(err.to_string())
                    })
                }
            }

            #[wasm_bindgen]
            impl [<$model_ty TopLevelWasm>] {
                pub fn attach_to_div(&mut self, div: web_sys::Element) -> Result<(), wasm_bindgen::JsValue> {
                    let obj = murrelet_wasm::interface::new_svg_obj()?;
                    self.svg_obj = Some(obj);

                    Ok(())
                }

                pub fn new_conf(conf: &str) -> Result<Self, wasm_bindgen::JsValue> {
                    Self::new_conf_livecode(conf).map_err(|err| {
                        wasm_bindgen::JsValue::from_str(&err.to_string())
                    })
                }

                pub fn new_conf_wrapper(conf: &[<$conf_ty Wrapper>]) -> Result<Self, wasm_bindgen::JsValue> {
                    Self::new_livecode(&conf.0).map_err(|err| {
                        wasm_bindgen::JsValue::from_str(&err.to_string())
                    })
                }

                pub fn set_config_json(&mut self, drawer_str: &str) -> Result<(), wasm_bindgen::JsValue> {
                    self.set_config_json_internal(drawer_str).map_err(|err| {
                        wasm_bindgen::JsValue::from_str(&err.to_string())
                    })
                }

                pub fn set_config(&mut self, conf: &[<$conf_ty Wrapper>]) -> Result<(), wasm_bindgen::JsValue> {
                    self.set_config_internal(conf).map_err(|err| {
                        wasm_bindgen::JsValue::from_str(&err.to_string())
                    })
                }

                pub fn rn_names(&self) -> Vec<String> {
                    $conf_ty::rn_names()
                }

                pub fn to_unclamped_dist(&self) -> Vec<f32> {
                    self.livecode.config().drawing.to_dist()
                }

                pub fn make_gui(&self) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
                    serde_json::to_string(&$conf_ty::make_gui())
                        .map(|a| wasm_bindgen::JsValue::from_str(&a))
                        .map_err(|err| {
                            wasm_bindgen::JsValue::from_str(&err.to_string())
                        })
                }

                pub fn tick(&mut self) {
                    let app_input = self.app_mng.tick();
                    self.livecode.update(&app_input, false).ok();

                    // send the config to the user's model... (maybe this can be Rc or something...)
                    self.model.set_conf(self.livecode.config().drawing.clone()); //hmm
                    self.model.update(&app_input);
                }

                pub fn draw_paths(&self, draw_opts_str: &str) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
                    let draw_opts: $draw_opts_ty = serde_json::from_str(&draw_opts_str).map_err(|err| {
                        wasm_bindgen::JsValue::from_str(&err.to_string())
                    })?;

                    let svg_draw_config = self.livecode.svg_save_path().with_no_resize();
                    let draw_ctx = murrelet_wasm::draw::WebSDrawCtx::new(&svg_draw_config);


                    let mixed_drawables = self.model.draw(&draw_opts);
                    draw_ctx.drawn_shapes(&mixed_drawables);

                    Ok(draw_ctx.make_html_jsvalue(Some(2)))
                }

                // update app_mng state and configs related to it

                pub fn set_bpm(&mut self, bpm: f32) {
                    self.app_mng.set_bpm(bpm);
                }

                pub fn set_beats_per_bar(&mut self, beats_per_bar: f32) {
                    self.app_mng.set_beats_per_bar(beats_per_bar);
                }

                pub fn set_window_dims(&mut self, x: f32, y: f32) {
                    self.app_mng.set_window_dims(x, y);
                }

                pub fn set_mouse_position(&mut self, x: f32, y: f32) {
                    self.app_mng.set_mouse_position(x, y);
                }

                pub fn set_mouse_left_is_down(&mut self) {
                    self.app_mng.set_mouse_left_is_down();
                }

                pub fn set_mouse_left_is_up(&mut self) {
                    self.app_mng.set_mouse_left_is_up();
                }

                pub fn set_custom_var(&mut self, key: String, value: f32) {
                    self.app_mng.set_custom_var(key, value);
                }
            }


            #[wasm_bindgen]
            pub struct [<$model_ty Gen>] {
                emb_gen: murrelet_gen::embedding::MemoizedEmbeddingGenerator,
            }

            #[wasm_bindgen]
            impl [<$model_ty Gen>] {
                #[wasm_bindgen(constructor)]
                pub fn new(digits: usize) -> Self {
                    Self {
                        emb_gen: murrelet_gen::embedding::MemoizedEmbeddingGenerator::new(digits, <$conf_ty>::rn_count()),
                    }
                }

                #[wasm_bindgen]
                pub fn from_gen_steps(&self, gen_str: &str) -> JsResult<[<$conf_ty Wrapper>]> {
                    let step = murrelet_gen::embedding::EmbeddingGenStep::parse_expr(gen_str).to_js()?;
                    let conf = step.compute(&self.emb_gen);
                    // at last! we have a spoonbill!
                    let conf = <$conf_ty>::from_dist(conf);

                    // wrap it in a wasm type
                    Ok([<$conf_ty Wrapper>](conf))
                }

                pub fn model_from_gen_steps(&self, gen_str: &str) -> JsResult<[<$model_ty TopLevelWasm>]> {
                    let conf = self.from_gen_steps(gen_str)?;
                    [<$model_ty TopLevelWasm>]::new_conf_wrapper(&conf)
                }
            }

            #[wasm_bindgen]
            pub fn new_generator(digits: usize) -> [<$model_ty Gen>] {
                [<$model_ty Gen>]::new(digits)
            }

            #[wasm_bindgen]
            pub fn new_model_from_conf(conf: &[<$conf_ty Wrapper>]) -> JsResult<[<$model_ty TopLevelWasm>]> {
                [<$model_ty TopLevelWasm>]::new_conf_wrapper(conf)
            }
        }

        // use this to get the names into javascript...
        #[wasm_bindgen]
        pub fn murrelet_export_info() -> String {
            format!(
                r#"{{"crate":"{}","top_level":"{}","gen":"{}","conf_wrapper":"{}"}}"#,
                env!("CARGO_PKG_NAME"),
                format!("{}TopLevelWasm", stringify!($model_ty)),
                format!("{}Gen", stringify!($model_ty)),
                format!("{}Wrapper", stringify!($conf_ty)),
            )
        }

    };
}

// hrm not sure about embedding ending up here...

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmEmbeddingGen(murrelet_gen::embedding::EmbeddingGenStep);

#[wasm_bindgen]
impl WasmEmbeddingGen {
    #[wasm_bindgen]
    pub fn new_emb(s: &str) -> JsResult<Self> {
        let emb = MurreletQuantizedEmbedding::from_str(s).to_js()?;
        Ok(Self(murrelet_gen::embedding::EmbeddingGenStep::emb(emb)))
    }

    #[wasm_bindgen]
    pub fn new_seed(s: u64) -> Self {
        Self(murrelet_gen::embedding::EmbeddingGenStep::seed(s))
    }

    #[wasm_bindgen]
    pub fn gauss(&self, rand_seed: u64, stdev: f32) -> Self {
        let source = Box::new(self.0.clone());
        Self(murrelet_gen::embedding::EmbeddingGenStep::NearSeedGaussian {
            source,
            rand_seed,
            stdev: stdev.into(),
        })
    }

    #[wasm_bindgen]
    pub fn rerand(&self, rand_seed: u64, rerand_chance: f32) -> Self {
        let source = Box::new(self.0.clone());
        Self(murrelet_gen::embedding::EmbeddingGenStep::NearSeedReRand {
            source,
            rand_seed,
            rerand_chance: rerand_chance.into(),
        })
    }

    #[wasm_bindgen]
    pub fn mix(&self, other: &Self, amount: f32) -> Self {
        let source_a = Box::new(self.0.clone());
        let source_b = Box::new(other.0.clone());
        Self(murrelet_gen::embedding::EmbeddingGenStep::Mix {
            source_a,
            source_b,
            mix: amount.into(),
        })
    }
}

pub const MURRELET_CLIENT_JS: &str = include_str!("../js/murrelet_client.js");

/// Generate a tiny per-crate JS wrapper that wires `<crate>.js` into `makeMurreletClient`.
///
/// `crate_js_module` should be something like `"./spoonbill.js"` (the wasm-pack JS glue filename).
/// `export_name` should be a JS identifier like `"spoonbill"` (no hyphens).
pub fn per_crate_client_js(crate_js_module: &str, export_name: &str) -> String {
    format!(
        r#"import * as wasm from "{crate_js_module}";
import {{ makeMurreletClient }} from "./murrelet_client.js";

export const {export_name} = makeMurreletClient(wasm);
export default {export_name};
"#,
        crate_js_module = crate_js_module,
        export_name = export_name,
    )
}

/// Turn a crate/package name into a safe JS identifier (e.g. `my-crate` -> `my_crate`).
pub fn js_ident_from_pkg_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}
