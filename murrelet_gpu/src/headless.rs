// claude help here
// Render a graphic to an off-screen texture and read it back to a PNG — the GPU
// twin of murrelet_svg's render_to_svg. No window: pair with a device made via
// OwnedDeviceState::new_from_native(). Extracted from portable_birdblend.
use std::error::Error;
use std::path::Path;

use image::ColorType;
#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;
#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

use crate::device_state::{DeviceStateForRender, OwnedDeviceState};
use crate::graphics_ref::{GraphicsRefCustom, GraphicsVertex, quick_texture};
use crate::window::GraphicsWindowConf;

// A GPU pipeline that can render one frame off-screen: run its passes, then hand
// back the graphic to read out. This is the single-conversion unit a headless
// PNG entry point (or a batch runner over many) drives.
pub trait IsHeadlessGraphic {
    type Vertex: GraphicsVertex;
    fn render_passes(&self, render_device: &DeviceStateForRender);
    fn output(&self) -> &GraphicsRefCustom<Self::Vertex>;

    /// Optional headless prep, run once before `render_passes`: fill any CPU-side
    /// drawer / sync GPU inputs from config — the per-frame `update()` work the
    /// windowed path does but the headless path otherwise skips. **Default no-op**,
    /// so pure-shader sketches need nothing and there's no separate arm to choose;
    /// drawer-fed sketches (lily, ethereal) override it. Forgetting to override just
    /// yields the pre-hook behavior (a thin/blank drawer), never a miscompile.
    fn prepare(&mut self, _c: &GraphicsWindowConf) {}
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
    let display = quick_texture(c.dims(), c.device());
    let display_view = display.default_view();
    let render_device = DeviceStateForRender::new(owned.to_borrowed(), display_view);
    graphic.render_passes(&render_device);
    save_graphic_png(c, graphic.output(), out_path)
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
    fn f16_to_u8(v: u16) -> u8 {
        (half::f16::from_bits(v).to_f32().clamp(0.0, 1.0) * 255.0).round() as u8
    }

    let dims = graphic.render_dims();
    let export_c = c.with_dims(dims);

    let rendered = quick_texture(dims, export_c.device.device());
    let rendered_view = rendered.default_view();

    graphic.render_to_texture(export_c.device, &rendered_view);

    let bytes_per_pixel = match rendered.desc.format {
        wgpu::TextureFormat::Rgba8Unorm => 4,
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
                texture: &rendered.texture,
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
            let row = &row[..unpadded_bytes_per_row as usize];
            match rendered.desc.format {
                wgpu::TextureFormat::Rgba8Unorm => pixels.extend_from_slice(row),
                wgpu::TextureFormat::Rgba16Float => {
                    let row_u16: &[u16] = bytemuck::cast_slice(row);
                    pixels.extend(row_u16.chunks_exact(4).flat_map(|px| {
                        [
                            f16_to_u8(px[0]),
                            f16_to_u8(px[1]),
                            f16_to_u8(px[2]),
                            f16_to_u8(px[3]),
                        ]
                    }));
                }
                _ => unreachable!(),
            }
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
