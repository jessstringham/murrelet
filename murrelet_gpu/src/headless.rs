// claude help here
// Render a graphic to an off-screen texture and read it back to a PNG — the GPU
// twin of murrelet_svg's render_to_svg. No window: pair with a device made via
// OwnedDeviceState::new_from_native(). Extracted from portable_birdblend.
use std::error::Error;
use std::path::Path;

use image::ColorType;
use murrelet_livecode::state::LivecodeWorldState;
#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;
#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

use crate::device_state::{DeviceStateForRender, OwnedDeviceState};
use crate::build_shader; // recurses by bare name, so import it directly
use crate::gpu_macros::ShaderStr; // build_shader! expands to an unqualified ShaderStr
use crate::graphics_ref::{
    GraphicsCreator, GraphicsRefCustom, GraphicsVertex, TextureAndDesc, quick_texture,
};
use crate::window::GraphicsWindowConf;

// A GPU pipeline that can render one frame off-screen: run its passes into the
// DISPLAY view, then the headless plumbing reads that view back. This is the
// single-conversion unit a headless PNG entry point (or a batch runner over
// many) drives. There's no `output()` to wire: capture reads the DISPLAY
// texture `render_passes` wrote, exactly the present target the windowed path
// shows — so the final `-> DISPLAY` pass is always what lands in the PNG.
pub trait IsHeadlessGraphic {
    fn render_passes(&self, render_device: &DeviceStateForRender);

    /// Optional headless prep, run once before `render_passes`: fill any CPU-side
    /// drawer / sync GPU inputs from config — the per-frame `update()` work the
    /// windowed path does but the headless path otherwise skips. **Default no-op**,
    /// so pure-shader sketches need nothing and there's no separate arm to choose;
    /// drawer-fed sketches (lily, ethereal) override it. Forgetting to override just
    /// yields the pre-hook behavior (a thin/blank drawer), never a miscompile.
    fn prepare(&mut self, _c: &GraphicsWindowConf) {}

    /// Optional per-frame state advance for the `nannou + gpu + stateful` arm: the
    /// arm loops `tick → prepare → render_passes` N times (the `--earlystop` count)
    /// so a GPU sketch whose CPU-side state accumulates (physics / growth / packing)
    /// can settle headless AND any feedback in its GPU pipeline (e.g. `res_feedback`)
    /// accumulates across the per-frame render_passes the way it would windowed.
    /// **Default no-op** (stateless gpu sketches inherit it — no separate arm to pick).
    /// A stateful gpu sketch stores its Drawing in the graphic and advances it here;
    /// `prepare` then fills the drawer from the settled state, and the per-frame
    /// `render_passes` from the arm drives the GPU side.
    fn tick(&mut self, _c: &GraphicsWindowConf, _world: &LivecodeWorldState) {}
}

// A window-free wgpu device, ready for headless rendering. The GPU analog of
// LiveCode::new being App-free.
pub fn new_native_device() -> OwnedDeviceState {
    pollster::block_on(OwnedDeviceState::new_from_native())
}

// Build an off-screen render target on `owned`'s device, run the graphic's passes
// into it, and save its output to a PNG. The window-free counterpart of nannou's
// view: pair with a device from new_native_device().
pub fn render_headless_graphic_to_png<G: IsHeadlessGraphic>(
    owned: &OwnedDeviceState,
    c: &GraphicsWindowConf,
    graphic: &G,
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let display = render_headless_graphic_passes(owned, c, graphic);
    capture_display_to_png(c, &display, out_path)
}

// Just the render half: build a fresh off-screen DISPLAY texture and run the
// graphic's passes once into it, returning that texture for readback. Safe to
// call repeatedly — feedback state lives inside the graphic's own ping-pong
// textures, not in the DISPLAY texture. Pair with `capture_display_to_png` for
// the final readback. Used by the `@headless_png_stateful` arm to loop
// tick→prepare→render_passes per frame so GPU feedback (e.g. `res_feedback`)
// accumulates the way windowed runs do; capture the returned texture from the
// last frame.
pub fn render_headless_graphic_passes<G: IsHeadlessGraphic>(
    owned: &OwnedDeviceState,
    c: &GraphicsWindowConf,
    graphic: &G,
) -> TextureAndDesc {
    let display = quick_texture(c.dims(), c.device());
    let display_view = display.default_view();
    let render_device = DeviceStateForRender::new(owned.to_borrowed(), display_view);
    graphic.render_passes(&render_device);
    display
}

// Just the capture half: read back the DISPLAY texture the passes rendered into
// and save it to PNG. Use after a `render_headless_graphic_passes` call — pass
// the texture it returned. This is the present target the windowed path shows,
// so the final `-> DISPLAY` pass is captured exactly.
pub fn capture_display_to_png(
    c: &GraphicsWindowConf,
    display: &TextureAndDesc,
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let (dims, pixels) = render_display_rgba8(c, display)?;
    image::save_buffer(out_path, &pixels, dims[0], dims[1], ColorType::Rgba8)?;
    Ok(())
}

fn align_copy_bytes_per_row(value: u32) -> u32 {
    if !value.is_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) {
        value + (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - (value % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
    } else {
        value
    }
}

pub fn render_graphic_rgba8<VertexKind: GraphicsVertex>(
    c: &GraphicsWindowConf,
    graphic: &GraphicsRefCustom<VertexKind>,
) -> Result<([u32; 2], Vec<u8>), Box<dyn Error>> {
    let dims = graphic.render_dims();
    let export_c = c.with_dims(dims);

    // Render into a linear working texture (DEFAULT_TEXTURE_FORMAT = Rgba16Float).
    let linear = quick_texture(dims, export_c.device.device());
    let linear_view = linear.default_view();
    graphic.render_to_texture(export_c.device, &linear_view);

    render_display_rgba8(&export_c, &linear)
}

// Read back an already-rendered linear DISPLAY texture (DEFAULT_TEXTURE_FORMAT =
// Rgba16Float) as sRGB-encoded rgba8. The shared back half of the readback path:
// the DISPLAY capture feeds its texture straight in; `render_graphic_rgba8`
// first renders a graphic into a linear texture, then delegates here.
pub fn render_display_rgba8(
    c: &GraphicsWindowConf,
    linear: &TextureAndDesc,
) -> Result<([u32; 2], Vec<u8>), Box<dyn Error>> {
    let dims = [linear.desc.size.width, linear.desc.size.height];
    let export_c = c.with_dims(dims);
    let linear_view = linear.default_view();

    // Final sRGB pass (BUG-L374). The windowed path presents through nannou's
    // sRGB swapchain, so the GPU applies the linear->sRGB OETF on store. The
    // headless readback must match or darks crush to black and midtones come out
    // too dark. Blit the linear texture through an identity shader whose target is
    // Rgba8UnormSrgb; the GPU encodes the gamma on store, exactly like the
    // windowed present-blit. (sRGB formats can't be STORAGE textures, so this
    // target uses a hand-rolled descriptor rather than quick_texture.)
    let rendered = export_c.device.device().create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: dims[0],
            height: dims[1],
            depth_or_array_layers: 1,
        },
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        label: Some("srgb readback target"),
        view_formats: &[],
    });
    let rendered_view = rendered.create_view(&wgpu::TextureViewDescriptor::default());

    let blit_shader: String = build_shader! {
        (
            raw r###"
                let result: vec4<f32> = textureSample(tex, tex_sampler, tex_coords);
            "###;
        )
    };
    let srgb_blit = GraphicsCreator::default()
        .with_dst_format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .to_graphics_ref(&export_c, "srgb_readback_blit", &blit_shader);
    srgb_blit.render_with_input_textures(export_c.device, &rendered_view, &linear_view, None);

    let bytes_per_pixel = match rendered.format() {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => 4,
        wgpu::TextureFormat::Rgba16Float => 8,
        format => {
            return Err(format!("unsupported texture format for png export: {format:?}").into());
        }
    };
    let unpadded_bytes_per_row = bytes_per_pixel * dims[0];
    let padded_bytes_per_row = align_copy_bytes_per_row(unpadded_bytes_per_row);
    const MAX_READBACK_BYTES: u64 = 128 * 1024 * 1024;
    let rows_per_chunk =
        ((MAX_READBACK_BYTES / padded_bytes_per_row as u64).max(1) as u32).min(dims[1]);

    let mut pixels = Vec::with_capacity((4 * dims[0] * dims[1]) as usize);

    for start_y in (0..dims[1]).step_by(rows_per_chunk as usize) {
        let chunk_rows = (dims[1] - start_y).min(rows_per_chunk);
        let buffer = export_c
            .device
            .device()
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("texture getter chunk"),
                size: (padded_bytes_per_row * chunk_rows) as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        let mut encoder =
            export_c
                .device
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Texture Copy Encoder"),
                });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &rendered,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: start_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBufferBase {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(chunk_rows),
                },
            },
            wgpu::Extent3d {
                width: dims[0],
                height: chunk_rows,
                depth_or_array_layers: 1,
            },
        );

        export_c.queue().submit(Some(encoder.finish()));

        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).expect("couldn't send map_async result")
        });
        export_c.device().poll(wgpu::Maintain::Wait);
        rx.recv()
            .expect("haven't received texture chunk")
            .map_err(|e| format!("failed to map texture chunk for readback: {e:?}"))?;

        let data = buffer_slice.get_mapped_range();
        for row in data
            .chunks(padded_bytes_per_row as usize)
            .take(chunk_rows as usize)
        {
            // The sRGB blit target is 8-bit; bytes are already gamma-encoded.
            let row = &row[..unpadded_bytes_per_row as usize];
            pixels.extend_from_slice(row);
        }
        drop(data);
        buffer.unmap();
    }

    Ok((dims, pixels))
}

pub fn save_graphic_png<VertexKind: GraphicsVertex>(
    c: &GraphicsWindowConf,
    graphic: &GraphicsRefCustom<VertexKind>,
    out_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let (dims, pixels) = render_graphic_rgba8(c, graphic)?;
    image::save_buffer(out_path, &pixels, dims[0], dims[1], ColorType::Rgba8)?;
    Ok(())
}
