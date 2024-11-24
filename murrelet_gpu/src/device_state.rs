use std::path::PathBuf;

use image::GenericImageView;

#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

use bytemuck::{Pod, Zeroable};
use half::f16;
use wgpu::util::DeviceExt;

use crate::graphics_ref::DEFAULT_LOADED_TEXTURE_FORMAT;

#[repr(transparent)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct F16U16(u16);

// use crate::graphics_ref::DEFAULT_LOADED_TEXTURE_FORMAT;
// const LOADED_TEXTURE_FORMAT = Rgba16Float;
// pub const LOADED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

// wrappers around ways of interacting with device/queue

#[derive(Debug)]
pub struct OwnedDeviceState {
    device: wgpu::Device,
    queue: wgpu::Queue,
}
impl OwnedDeviceState {
    pub fn to_borrowed<'a>(&'a self) -> DeviceState<'a> {
        DeviceState {
            device: self.device(),
            queue: self.queue(),
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

#[derive(Debug)]
pub struct DeviceState<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl<'a> DeviceState<'a> {
    pub fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        Self { device, queue }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

impl OwnedDeviceState {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        OwnedDeviceState { device, queue }
    }

    pub async fn new_from_native() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            ..Default::default()
        });

        let adapter = wgpu::util::initialize_adapter_from_env_or_default(&instance, None)
            .await
            .expect("failed to get adapter");

        // let adapter_limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    #[cfg(feature = "nannou")]
                    features: adapter.features(),
                    #[cfg(feature = "nannou")]
                    limits: adapter.limits(),
                    #[cfg(not(feature = "nannou"))]
                    required_features: adapter.features(),
                    #[cfg(not(feature = "nannou"))]
                    required_limits: adapter.limits(),
                    label: Some("Compute/RenderPass Device"),
                },
                None,
            )
            .await
            .expect("Failed to create device and queue");

        Self {
            // adapter_info,
            device,
            queue,
        }
    }
}

// borrowing from bevy
pub fn align_byte_size(value: u32) -> u32 {
    value + (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - (value % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
}

pub fn check_img_size(path: &PathBuf) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let img_rgba = img.to_rgba8();
    let (img_width, img_height) = img.dimensions();

    Ok((img_rgba.to_vec(), img_width, img_height))
}

fn convert_u8_to_f16u16_buffer(img_rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    // Convert u8 data to F16U16, scaling to [0.0, 1.0]
    let img_data_f16u16: Vec<F16U16> = img_rgba
        .chunks(4) // assuming RGBA data
        .flat_map(|pixel| {
            pixel.iter().map(|&c| {
                let f = f16::from_f32(c as f32 / 255.0);
                F16U16(f.to_bits())
            })
        })
        .collect();

    // Calculate row padding for 256-byte alignment
    let bytes_per_pixel = std::mem::size_of::<F16U16>() * 4; // 4 channels per pixel
    let unpadded_bytes_per_row = bytes_per_pixel * width as usize;
    let padded_bytes_per_row = ((unpadded_bytes_per_row + 255) / 256) * 256;
    let padding_per_row = padded_bytes_per_row - unpadded_bytes_per_row;

    // Prepare the padded image data
    let mut padded_img_data = Vec::with_capacity(padded_bytes_per_row * height as usize);

    let row_size = 4 * width as usize;

    for row in img_data_f16u16.chunks(row_size) {
        // Convert `&[F16U16]` to `&[u8]`
        let row_bytes: &[u8] = bytemuck::cast_slice(row);
        padded_img_data.extend_from_slice(row_bytes);
        // Add padding
        padded_img_data.extend(std::iter::repeat(0u8).take(padding_per_row));
    }

    padded_img_data
}

// todo, refactor reuse the img..
fn write_png_to_texture(
    device_state: &DeviceState,
    path: &PathBuf,
    texture: &wgpu::Texture,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the image
    let img = image::open(path)?;
    let img_rgba = img.to_rgba8();
    let (img_width, img_height) = img.dimensions();

    let bytes_per_row = 4 * img_width;
    // this is supposed to help if it's too small..
    let padded_row = align_byte_size(bytes_per_row);
    // let row_padding = 0;
    let buffer_rows = img_height;

    println!("img_width {:?}", img_width);
    println!("img_height {:?}", img_height);
    println!("buffer_rows {:?}", buffer_rows);

    // just get the name to name the texture
    let p = path.file_name().map(|x| x.to_str()).flatten().unwrap_or("");

    // let img_data_f16: Vec<F16> = img_rgba
    //     .pixels()
    //     .flat_map(|p| {
    //         p.0.iter().map(|&c| F16(f16::from_f32(c as f32 / 255.0)))
    //     })
    //     .collect();

    // let padded_img = convert_u8_to_f16u16_buffer(&img_rgba, img_width, img_height);

    // bah, uh, okay copy this to a buffer of the right length
    let mut padded_img = vec![0; (padded_row * buffer_rows).try_into().unwrap()];
    // for (row_i, data) in img_rgba.chunks(bytes_per_row as usize).enumerate() {
    for (row_i, data) in img_rgba.chunks(bytes_per_row as usize).enumerate() {
        let start = row_i * padded_row as usize;
        let end = start + data.len();

        // if row_i % 100 == 0 {
        //     println!("data {:?}", data);
        // }

        padded_img[start..end].copy_from_slice(data);
    }

    // buffer for loading the png
    let buffer = device_state
        .device()
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("texture from {}", p)),
            contents: &padded_img,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

    let mut encoder = device_state
        .device()
        .create_command_encoder(&Default::default());

    encoder.copy_buffer_to_texture(
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row), //Some(bytes_per_row + row_padding), // rgba
                rows_per_image: Some(buffer_rows),
            },
        },
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::Extent3d {
            width: img_width,
            height: img_height,
            depth_or_array_layers: 1,
        },
    );

    // and submit it!
    device_state.queue().submit(Some(encoder.finish()));

    Ok(())
}

#[derive(Clone, Debug)]
pub enum GraphicsAssets {
    Nothing,
    LocalFilesystem(PathBuf),
}
impl GraphicsAssets {
    pub fn local_filesystem(path: PathBuf) -> GraphicsAssets {
        GraphicsAssets::LocalFilesystem(path)
    }

    pub fn to_format(&self, default: wgpu::TextureFormat) -> wgpu::TextureFormat {
        match self {
            GraphicsAssets::Nothing => default,
            GraphicsAssets::LocalFilesystem(_) => DEFAULT_LOADED_TEXTURE_FORMAT,
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            GraphicsAssets::Nothing => true,
            _ => false,
        }
    }

    pub(crate) fn force_path_buf(&self) -> PathBuf {
        match self {
            GraphicsAssets::Nothing => panic!("expected path!"),
            GraphicsAssets::LocalFilesystem(p) => p.clone(),
        }
    }

    // #[cfg(feature = "nannou")]
    // pub fn to_texture(&self, c: &GraphicsWindowConf, device: &wgpu::Device, first_format: wgpu::TextureFormat) -> wgpu::Texture {
    //     let input_texture = match self {
    //         GraphicsAssets::LocalFilesystem(path) => {
    //             wgpu::Texture::from_path(c.window, path).unwrap() // load the path
    //         }
    //         GraphicsAssets::Nothing => Graphics::texture(c.dims, device, first_format),
    //     };
    // }

    // #[cfg(not(feature = "nannou"))]
    // pub fn to_texture(&self, c: &GraphicsWindowConf, device: &wgpu::Device, first_format: wgpu::TextureFormat) -> wgpu::Texture {
    //     let input_texture = match self {
    //         GraphicsAssets::LocalFilesystem(_) => {
    //             panic!("can't use local filesystem")
    //         }
    //         GraphicsAssets::Nothing => Graphics::texture(c.dims, device, first_format),
    //     };
    // }

    // i don't really think this works..
    pub(crate) fn maybe_load_texture(
        &self,
        device_state: &DeviceState,
        input_texture: &wgpu::Texture,
    ) {
        match self {
            GraphicsAssets::Nothing => {}
            GraphicsAssets::LocalFilesystem(path) => {
                write_png_to_texture(device_state, path, input_texture).ok();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GraphicsWindowConf<'a> {
    pub device: &'a DeviceState<'a>,
    pub dims: [u32; 2],
    pub assets_path: GraphicsAssets,
}
impl<'a> GraphicsWindowConf<'a> {
    pub fn new(
        device: &'a DeviceState,
        dims: [u32; 2],
        assets_path: GraphicsAssets,
    ) -> GraphicsWindowConf<'a> {
        GraphicsWindowConf {
            device,
            dims,
            assets_path,
        }
    }

    pub fn multi(&self, multiplier: f32) -> GraphicsWindowConf {
        let [x, y] = self.dims;
        GraphicsWindowConf {
            device: self.device,
            dims: [
                (x as f32 * multiplier) as u32,
                (y as f32 * multiplier) as u32,
            ],
            assets_path: GraphicsAssets::Nothing,
        }
    }

    pub fn dims(&self) -> [u32; 2] {
        self.dims
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device.device()
    }

    pub fn with_dims(&self, dims: [u32; 2]) -> Self {
        Self {
            dims,
            ..self.clone()
        }
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.device.queue()
    }
}

// new type just to pull in things available at render time
pub struct DeviceStateForRender<'a> {
    device_state: DeviceState<'a>,
    display_view: wgpu::TextureView,
}
impl<'a> DeviceStateForRender<'a> {
    pub fn new(device_state: DeviceState<'a>, display_view: wgpu::TextureView) -> Self {
        Self {
            device_state,
            display_view,
        }
    }

    pub fn device_state(&self) -> &DeviceState {
        &self.device_state
    }

    pub fn display_view(&self) -> &wgpu::TextureView {
        &self.display_view
    }
}
