use std::collections::HashMap;
#[allow(dead_code)]
use std::path::PathBuf;

use image::GenericImageView;

#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

use wgpu::util::DeviceExt;

use crate::graphics_ref::DEFAULT_LOADED_TEXTURE_FORMAT;

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

    if value % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT != 0 {
        value + (wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - (value % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
    } else {
        value
    }

}

pub fn check_img_size(path: &PathBuf) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let img_rgba = img.to_rgba8();
    let (img_width, img_height) = img.dimensions();

    Ok((img_rgba.to_vec(), img_width, img_height))
}

// todo, refactor reuse the img..
fn write_png_to_texture(
    device_state: &DeviceState,
    path: &PathBuf,
    texture: &wgpu::Texture,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the image
    println!("loading file {:?}", path);
    let img = image::open(path)?;

    println!("img.color() {:?}", img.color());

    let img_rgba = img.to_rgba8();
    let (img_width, img_height) = img.dimensions();

    let bytes_per_row = 4 * img_width;
    // this is supposed to help if it's too small..
    let padded_row = align_byte_size(bytes_per_row);
    // let row_padding = 0;
    let buffer_rows = img_height;

    println!("img_width {:?}", img_width);
    println!("img_height {:?}", img_height);
    println!("padded_row {:?}", padded_row);
    println!("buffer_rows {:?}", buffer_rows);

    // just get the name to name the texture
    let p = path.file_name().map(|x| x.to_str()).flatten().unwrap_or("");

    // bah, uh, okay copy this to a buffer of the right length
    let mut padded_img = vec![0; (padded_row * buffer_rows).try_into().unwrap()];
    for (row_i, data) in img_rgba.chunks(bytes_per_row as usize).enumerate() {
        let start = row_i * padded_row as usize;
        let end = start + data.len();

        padded_img[start..end].copy_from_slice(data);
    }


    let mut hist = HashMap::new();
    for value in &padded_img {
        *hist.entry(value).or_insert(0) += 1;
    }

    println!("hist {:?}", hist);

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
