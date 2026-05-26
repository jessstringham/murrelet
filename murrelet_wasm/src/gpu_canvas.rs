//! Multi-canvas wgpu support for the web: bind a wgpu surface to each named
//! DOM `<canvas>` and render a pipeline into it. Rust owns the canvas layout
//! (see [`GraphicsMng::make_img_defs`]).

use murrelet_common::StrId;
use murrelet_gpu::device_state::{
    DeviceStateForRender, GraphicsAssets, OwnedDeviceState,
};
use murrelet_gpu::gpu_macros::GPUPipeline;
use murrelet_gpu::window::GraphicsWindowConf;
use wasm_bindgen::JsValue;

/// Anything that can render a frame into the `display_view` carried by a
/// [`DeviceStateForRender`]. `GPUPipeline` already satisfies this, so a
/// `Box<dyn RenderToDisplay>` lets a [`GraphicsMng`] hold differently-typed
/// pipelines side by side.
pub trait RenderToDisplay {
    fn render(&self, device: &DeviceStateForRender);
}

impl<C> RenderToDisplay for GPUPipeline<C> {
    fn render(&self, device: &DeviceStateForRender) {
        GPUPipeline::render(self, device)
    }
}

/// One canvas: its DOM id, the device/surface bound to it, and the pipeline
/// that renders into it.
pub struct CanvasShader {
    canvas_id: StrId,
    device_state: OwnedDeviceState,
    surface: wgpu::Surface<'static>,
    pipeline: Box<dyn RenderToDisplay>,
}

impl CanvasShader {
    /// Resolve-or-create the canvas by id, bind a wgpu surface to it, then let
    /// `make_pipeline` build the pipeline against the freshly-created device.
    pub async fn new<F>(
        canvas_id: StrId,
        dims: [u32; 2],
        make_pipeline: F,
    ) -> Result<Self, JsValue>
    where
        F: FnOnce(&GraphicsWindowConf) -> Box<dyn RenderToDisplay>,
    {
        let (device_state, surface) =
            OwnedDeviceState::new_from_web(canvas_id.as_str(), dims).await?;

        let pipeline = {
            let d = device_state.to_borrowed();
            let conf = GraphicsWindowConf::new(&d, dims, GraphicsAssets::Nothing);
            make_pipeline(&conf)
        };

        Ok(Self {
            canvas_id,
            device_state,
            surface,
            pipeline,
        })
    }

    pub fn canvas_id(&self) -> StrId {
        self.canvas_id
    }

    pub fn draw(&self) -> Result<(), JsValue> {
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("get_current_texture: {e}")))?;
        let view = frame.texture.create_view(&Default::default());

        let dsr = DeviceStateForRender::new(self.device_state.to_borrowed(), view);
        self.pipeline.render(&dsr);

        frame.present();
        Ok(())
    }
}

/// Holds every canvas the model drives. Rust owns the canvas list, so it can
/// also emit the HTML mount points via [`GraphicsMng::make_img_defs`].
#[derive(Default)]
pub struct GraphicsMng {
    canvas_shaders: Vec<CanvasShader>,
}

impl GraphicsMng {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, shader: CanvasShader) {
        self.canvas_shaders.push(shader);
    }

    pub fn canvas_ids(&self) -> Vec<String> {
        self.canvas_shaders
            .iter()
            .map(|s| s.canvas_id().as_str().to_owned())
            .collect()
    }

    pub fn draw(&self) {
        for c in &self.canvas_shaders {
            if let Err(e) = c.draw() {
                web_sys::console::error_1(&e);
            }
        }
    }

    /// The `<image>` mount-point defs for these canvases, joined into one
    /// string. Delegates to the existing `murrelet_svg::make_canvas_imgs`.
    pub fn make_img_defs(&self) -> String {
        murrelet_svg::svg::make_canvas_imgs(&self.canvas_ids())
            .into_iter()
            .map(|img| img.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
