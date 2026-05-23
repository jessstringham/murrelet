use std::fmt::Display;

use glam::vec2;
use murrelet_common::{MurreletAppInput, ToStrId};
use murrelet_gen::embedding::MurreletQuantizedEmbedding;
use murrelet_perform::AppConfig;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};
use itertools::Itertools;
pub use murrelet_perform::interface::{IsDrawableMurreletModel, IsMurreletModel};

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


// just so we can manage these things outside of a macro...
pub struct AppManager {
    pub conf: AppConfig,
    pub state: MurreletAppInput,
}
impl Default for AppManager {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn set_custom_var<S: ToStrId>(&mut self, key: S, value: f32) {
        self.state.custom_vars.insert(key.to_strid(), value);
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

    pub fn frame(&self) -> u64 {
        self.state.elapsed_frames
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
    let document: Document = window()
        .ok_or("no window")?
        .document()
        .ok_or("no document")?;

    let svg_ns = Some("http://www.w3.org/2000/svg");

    let svg_el = document
        .create_element_ns(svg_ns, "svg")?
        .dyn_into::<SvgsvgElement>()?;
    svg_el.set_attribute("viewBox", "0 0 800 800")?;

    let paths_g_el = document
        .create_element_ns(svg_ns, "g")?
        .dyn_into::<SvgElement>()?;
    paths_g_el.set_id("paths");

    svg_el.append_child(&paths_g_el)?;

    Ok(MurreletSvgObj { svg_el, paths_g_el })
}

#[macro_export]
macro_rules! basic_wrapper_conf_only {
    ($conf_ty:ident, $conf_ty_wrapper:ident, $ctrl_conf_ty:ty) => {
        #[wasm_bindgen::prelude::wasm_bindgen]
        pub struct $conf_ty_wrapper($conf_ty);

        impl LivecodeToControl<$ctrl_conf_ty> for $conf_ty_wrapper {
            fn to_control(&self) -> $ctrl_conf_ty {
                self.0.to_control()
            }
        }
    };
}

#[macro_export]
macro_rules! basic_wrapper {
    ($model_ty:ident, $conf_ty:ident, $conf_ty_wrapper:ident, $ctrl_conf_ty:ty, $conf_top_level:ident, $ctrl_conf_top_level:ident, $model_wasm:ident) => {
        use murrelet_livecode::livecode::LivecodeToControl;
        use murrelet_perform::TopLevelLiveCodeJson;
        use murrelet_perform::perform::{
            ControlAppConfig, ControlLazyAppConfig, LazyAppConfig, WithDrawerUpdator,
        };

        basic_wrapper_conf_only!($conf_ty, $conf_ty_wrapper, $ctrl_conf_ty);

        #[derive(Debug, Clone, Livecode, Lerpable, TopLevelLiveCodeJson)]
        pub struct $conf_top_level {
            pub drawing: $conf_ty,
            pub app: murrelet_perform::AppConfig,
        }
        impl $conf_top_level {
            fn new(model: &$model_ty, app_mng: &murrelet_wasm::interface::AppManager) -> Self {
                Self {
                    drawing: model.get_conf().clone(),
                    app: app_mng.conf().clone(),
                }
            }
        }

        impl WithDrawerUpdator<$ctrl_conf_ty> for $ctrl_conf_top_level {
            fn new_from_parts(
                app: murrelet_perform::ControlAppConfig,
                drawing: $ctrl_conf_ty,
            ) -> Self {
                $ctrl_conf_top_level { app, drawing }
            }

            fn parse_drawer(text: &str) -> murrelet_livecode::types::LivecodeResult<$ctrl_conf_ty> {
                serde_json::from_str(&text).map_err(|err| {
                    murrelet_livecode::types::LivecodeError::JsonParse(err.to_string())
                })
            }

            fn set_drawing_conf(&mut self, drawing_conf: $ctrl_conf_ty) {
                self.drawing = drawing_conf;
            }

            fn set_app(&mut self, app: murrelet_perform::ControlAppConfig) {
                self.app = app;
            }
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        pub struct $model_wasm {
            livecode: LiveCode,
            model: $model_ty,
            app_mng: murrelet_wasm::interface::AppManager,
            svg_obj: Option<MurreletSvgObj>,
        }

        // non-wasm things
        impl $model_wasm {
            pub fn new_conf_livecode(conf: &str) -> murrelet_livecode::types::LivecodeResult<Self> {
                let app_mng = murrelet_wasm::interface::AppManager::new();

                let control_config = $ctrl_conf_top_level::from_json(&app_mng.conf, conf)?;
                let livecode = LiveCode::new_wasm(control_config)?;

                let model = $model_ty::init(livecode.config().drawing.clone());

                Ok(Self {
                    model,
                    app_mng,
                    livecode,
                    svg_obj: None,
                })
            }

            pub fn new_livecode(conf: &$conf_ty) -> murrelet_livecode::types::LivecodeResult<Self> {
                let app_mng = murrelet_wasm::interface::AppManager::new();

                let control_config = $ctrl_conf_top_level::from_regular(&app_mng.conf, conf)?;
                let livecode = LiveCode::new_wasm(control_config)?;

                let model = $model_ty::init(livecode.config().drawing.clone());

                Ok(Self {
                    model,
                    app_mng,
                    livecode,
                    svg_obj: None,
                })
            }

            pub fn set_config_json_internal(
                &mut self,
                drawer_str: &str,
            ) -> murrelet_livecode::types::LivecodeResult<()> {
                let control_config =
                    $ctrl_conf_top_level::from_json(&self.app_mng.conf, drawer_str)?;

                self.livecode.update_config_directly(control_config)?;

                Ok(())
            }

            pub fn set_config_internal(
                &mut self,
                drawer_str: &$conf_ty_wrapper,
            ) -> murrelet_livecode::types::LivecodeResult<()> {
                let control_config =
                    $ctrl_conf_top_level::from_regular(&self.app_mng.conf, drawer_str)?;

                self.livecode.update_config_directly(control_config)?;

                Ok(())
            }
        }

        // wasm things
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $model_wasm {
            pub fn new_conf(conf: &str) -> Result<Self, wasm_bindgen::JsValue> {
                Self::new_conf_livecode(conf)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
            }

            pub fn new_conf_wrapper(
                conf: &$conf_ty_wrapper,
            ) -> Result<Self, wasm_bindgen::JsValue> {
                Self::new_livecode(&conf.0)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen]
            pub fn frame(&self) -> u64 {
                self.app_mng.frame()
            }

            #[wasm_bindgen::prelude::wasm_bindgen]
            pub fn set_config_json(
                &mut self,
                drawer_str: &str,
            ) -> Result<(), wasm_bindgen::JsValue> {
                self.set_config_json_internal(drawer_str)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
            }

            pub fn set_config(
                &mut self,
                conf: &$conf_ty_wrapper,
            ) -> Result<(), wasm_bindgen::JsValue> {
                self.set_config_internal(conf)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
            }

            pub fn get_config_json(&self) -> Result<String, wasm_bindgen::JsValue> {
                let conf = &self.livecode.config().drawing;
                serde_json::to_string(conf)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
            }

            pub fn tick(&mut self) {
                let app_input = self.app_mng.tick();
                self.livecode.update(&app_input, false).ok();

                // send the config to the user's model... (maybe this can be Rc or something...)
                self.model.set_conf(self.livecode.config().drawing.clone()); //hmm
                self.model.update(&app_input);
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

            #[wasm_bindgen::prelude::wasm_bindgen]
            pub fn set_custom_var(&mut self, key: String, value: f32) {
                self.app_mng.set_custom_var(key, value);
            }
        }

        // use this to get the names into javascript...
        #[wasm_bindgen::prelude::wasm_bindgen]
        pub fn murrelet_export_info() -> String {
            format!(
                r#"{{"crate":"{}","top_level":"{}","gen":"{}","conf_wrapper":"{}"}}"#,
                env!("CARGO_PKG_NAME"),
                format!("{}TopLevelWasm", stringify!($model_ty)),
                format!("{}Gen", stringify!($model_ty)),
                format!("{}Wrapper", stringify!($conf_ty)),
            )
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        pub fn new_model_from_conf(
            conf: &$conf_ty_wrapper,
        ) -> Result<$model_wasm, wasm_bindgen::JsValue> {
            $model_wasm::new_conf_wrapper(conf)
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        pub fn new_model(conf: &str) -> Result<$model_wasm, wasm_bindgen::JsValue> {
            $model_wasm::new_conf(conf)
        }
    };
}

#[macro_export]
macro_rules! bonus_draw_wrapper {
    ($model_ty:ty, $conf_ty:ty, $conf_ty_wrapper:ty, $ctrl_conf_ty:ty, $conf_top_level:ty, $ctrl_conf_top_level:ty, $model_wasm:ty, $draw_opts_ty:ty) => {
        use web_sys::{Document, Element, SvgElement, SvgsvgElement, Window};

        impl $model_wasm {
            pub fn parse_draw_opts(
                conf: &str,
            ) -> murrelet_livecode::types::LivecodeResult<$draw_opts_ty> {
                serde_json::from_str(&conf).map_err(|err| {
                    murrelet_livecode::types::LivecodeError::JsonParse(err.to_string())
                })
            }
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $model_wasm {
            pub fn attach_to_div(&mut self) -> Result<(), wasm_bindgen::JsValue> {
                let obj = murrelet_wasm::interface::new_svg_obj()?;
                self.svg_obj = Some(obj);

                Ok(())
            }

            pub fn draw_paths(
                &self,
                draw_opts_str: &str,
            ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
                let draw_opts: $draw_opts_ty = serde_json::from_str(&draw_opts_str)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))?;

                let svg_draw_config = self.livecode.svg_save_path().with_no_resize();
                let draw_ctx = murrelet_wasm::draw::WebSDrawCtx::new(&svg_draw_config);

                let mixed_drawables = self
                    .model
                    .draw(&draw_opts)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))?;
                draw_ctx.drawn_shapes(&mixed_drawables);

                Ok(draw_ctx.make_html_jsvalue(Some(2)))
            }
        }
    };
}

#[macro_export]
macro_rules! bonus_gui_wrapper {
    ($model_ty:ty, $conf_ty:ty, $conf_ty_wrapper:ty, $ctrl_conf_ty:ty, $conf_top_level:ty, $ctrl_conf_top_level:ty, $model_wasm:ty) => {
        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $model_wasm {
            pub fn make_gui(&self) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
                serde_json::to_string(&<$conf_ty as murrelet_gui::CanMakeGUI>::make_gui())
                    .map(|a| wasm_bindgen::JsValue::from_str(&a))
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
            }
        }
    };
}

#[macro_export]
macro_rules! bonus_embedding_wrapper {
    ($model_ty:ty, $conf_ty:ty, $conf_ty_wrapper:ident, $ctrl_conf_ty:ty, $conf_top_level:ty, $ctrl_conf_top_level:ty, $model_wasm:ident, $conf_gen_manager:ident) => {
        bonus_embedding_wrapper_conf_gen_only!(
            $conf_ty,
            $conf_ty_wrapper,
            $ctrl_conf_ty,
            $conf_gen_manager
        );

        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $model_wasm {
            pub fn from_gen_steps(
                manager: &$conf_gen_manager,
                gen_str: &str,
            ) -> Result<$model_wasm, wasm_bindgen::JsValue> {
                let conf = manager.from_gen_steps(gen_str)?;
                $model_wasm::new_conf_wrapper(&conf)
            }
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $model_wasm {
            pub fn rn_names(&self) -> Vec<String> {
                <$conf_ty as murrelet_gen::CanSampleFromDist>::rn_names()
            }

            // JSON array of RnSpec, parallel to rn_names() — gen method + params per rn slot.
            pub fn rn_specs(&self) -> Result<String, wasm_bindgen::JsValue> {
                serde_json::to_string(
                    &<$conf_ty as murrelet_gen::CanSampleFromDist>::rn_specs(),
                )
                .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))
            }

            pub fn to_unclamped_dist(&self) -> Vec<f32> {
                <_ as murrelet_gen::CanSampleFromDist>::to_dist(self.model.get_conf())
            }

            pub fn to_clamped_dist(&self, digits: usize) -> Result<String, wasm_bindgen::JsValue> {
                let dist = <_ as murrelet_gen::CanSampleFromDist>::to_dist(self.model.get_conf());

                let quantized =
                    murrelet_gen::embedding::MurreletQuantizedEmbedding::from_rn(&dist, digits);
                let encoded = quantized
                    .encode()
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))?;
                Ok(encoded.to_string())
            }
        }
    };
}

#[macro_export]
macro_rules! bonus_embedding_wrapper_conf_gen_only {
    ($conf_ty:ty, $conf_ty_wrapper:ident, $ctrl_conf_ty:ty, $conf_gen_manager:ident) => {
        #[wasm_bindgen::prelude::wasm_bindgen]
        pub struct $conf_gen_manager {
            emb_gen: murrelet_gen::embedding::MemoizedEmbeddingGenerator,
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $conf_gen_manager {
            #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
            pub fn new(digits: usize) -> $conf_gen_manager {
                $conf_gen_manager {
                    emb_gen: murrelet_gen::embedding::MemoizedEmbeddingGenerator::new(
                        digits,
                        <$conf_ty as murrelet_gen::CanSampleFromDist>::rn_count(),
                    ),
                }
            }

            #[wasm_bindgen::prelude::wasm_bindgen]
            pub fn from_gen_steps(
                &self,
                gen_str: &str,
            ) -> Result<$conf_ty_wrapper, wasm_bindgen::JsValue> {
                let step = murrelet_gen::embedding::EmbeddingGenStep::parse_expr(gen_str)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err.to_string()))?;
                let conf = step.compute(&self.emb_gen);
                // at last! we have a spoonbill!
                let conf = <$conf_ty as murrelet_gen::CanSampleFromDist>::from_dist(conf);

                // wrap it in a wasm type
                Ok($conf_ty_wrapper(conf))
            }
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        impl $conf_ty_wrapper {
            pub fn rn_names() -> Vec<String> {
                <$conf_ty as murrelet_gen::CanSampleFromDist>::rn_names()
            }

            pub fn to_unclamped_dist(&self) -> Vec<f32> {
                <_ as murrelet_gen::CanSampleFromDist>::to_dist(&self.0)
            }
        }
    };
}

// chatgpt can write my macro
#[macro_export]
macro_rules! export_murrelet_web_model {
    // Entry: Model<Conf> + features...
    ($model_ty:ident < $conf_ty:ident > $($rest:tt)*) => {
        $crate::paste::paste! {
            basic_wrapper!(
                $model_ty,
                $conf_ty,
                [<$conf_ty Wrapper>],
                [<Control $conf_ty>],
                [<$conf_ty TopLevelLivecode>],
                [<Control $conf_ty TopLevelLivecode>],
                [<$model_ty TopLevelWasm>]
            );
        }

        $crate::export_murrelet_web_model!(@parse $model_ty, $conf_ty, $($rest)*);
    };

    // special case for just doing a conf emb
    (< $conf_ty:ident > + emb) => {
        $crate::paste::paste! {

            basic_wrapper_conf_only!($conf_ty, [<$conf_ty Wrapper>], [<Control $conf_ty>]);

            bonus_embedding_wrapper_conf_gen_only!(
                $conf_ty,
                [<$conf_ty Wrapper>],
                [<Control $conf_ty>],
                [<$conf_ty Gen>]
            );
        }
    };


    (@parse $model_ty:ident, $conf_ty:ident,) => {};
    (@parse $model_ty:ident, $conf_ty:ident) => {};

    // + gui
    (@parse $model_ty:ident, $conf_ty:ident, + gui $($tail:tt)*) => {
        $crate::paste::paste! {
            bonus_gui_wrapper!(
                $model_ty,
                $conf_ty,
                [<$conf_ty Wrapper>],
                [<Control $conf_ty>],
                [<$conf_ty TopLevelLivecode>],
                [<Control $conf_ty TopLevelLivecode>],
                [<$model_ty TopLevelWasm>]
            );
        }
        $crate::export_murrelet_web_model!(@parse $model_ty, $conf_ty, $($tail)*);
    };

    // + emb
    (@parse $model_ty:ident, $conf_ty:ident, + emb $($tail:tt)*) => {
        $crate::paste::paste! {
            bonus_embedding_wrapper!(
                $model_ty,
                $conf_ty,
                [<$conf_ty Wrapper>],
                [<Control $conf_ty>],
                [<$conf_ty TopLevelLivecode>],
                [<Control $conf_ty TopLevelLivecode>],
                [<$model_ty TopLevelWasm>],
                [<$conf_ty Gen>]
            );
        }
        $crate::export_murrelet_web_model!(@parse $model_ty, $conf_ty, $($tail)*);
    };

    // + draw(MyDrawOpts)
    (@parse $model_ty:ident, $conf_ty:ident, + draw($draw_opts_ty:ty) $($tail:tt)*) => {
        $crate::paste::paste! {
            bonus_draw_wrapper!(
                $model_ty,
                $conf_ty,
                [<$conf_ty Wrapper>],
                [<Control $conf_ty>],
                [<$conf_ty TopLevelLivecode>],
                [<Control $conf_ty TopLevelLivecode>],
                [<$model_ty TopLevelWasm>],
                $draw_opts_ty
            );
        }
        $crate::export_murrelet_web_model!(@parse $model_ty, $conf_ty, $($tail)*);
    };

    // Friendlier error if someone writes an unknown feature
    (@parse $model_ty:ident, $conf_ty:ident, + $unknown:tt $($tail:tt)*) => {
        compile_error!("export_murrelet_web_model!: unknown feature. Use + gui, + emb, or + draw(Type).");
        $crate::export_murrelet_web_model!(@parse $model_ty, $conf_ty, $($tail)*);
    };

    // Catch-all (usually missing a leading '+')
    (@parse $model_ty:ident, $conf_ty:ident, $($bad:tt)+) => {
        compile_error!("export_murrelet_web_model!: expected '+ gui', '+ emb', or '+ draw(Type)'.");
    };
}

// hrm not sure about embedding ending up here...

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmEmbeddingGen(murrelet_gen::embedding::EmbeddingGenStep);

#[wasm_bindgen]
impl WasmEmbeddingGen {
    #[wasm_bindgen]
    pub fn new_emb(s: &str) -> Result<Self, wasm_bindgen::JsValue> {
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
        Self(
            murrelet_gen::embedding::EmbeddingGenStep::NearSeedGaussian {
                source,
                rand_seed,
                stdev: stdev.into(),
            },
        )
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

    #[wasm_bindgen]
    pub fn to_expr_string(&self) -> String {
        self.0.to_expr_string().unwrap()
    }

    #[wasm_bindgen]
    pub fn lock_indices(&self, lock_with: &Self, indices: Vec<usize>) -> Self {
        let source = Box::new(self.0.clone());
        let overwrite_with = Box::new(lock_with.0.clone());
        let mut sorted_i = indices.clone();
        sorted_i.sort();
        let lock_indices = sorted_i.into_iter().map(|x| x.to_string()).join(",");

        Self(murrelet_gen::embedding::EmbeddingGenStep::Lock {
            source,
            overwrite_with,
            lock_indices,
        })
    }
}

pub const MURRELET_CLIENT_JS: &str = include_str!("../js/murrelet_client.js");

// chatgpt
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
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
