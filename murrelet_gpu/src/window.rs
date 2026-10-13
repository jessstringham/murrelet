// stores info about the window..

use crate::{
    device_state::{DeviceState, GraphicsAssets},
    graphics_ref::DEFAULT_TEXTURE_FORMAT,
};

#[derive(Clone, Debug)]
pub struct GraphicsWindowConf<'a> {
    pub device: &'a DeviceState<'a>,
    pub dims: [u32; 2],
    pub assets_path: GraphicsAssets,
    pub surface_format: wgpu::TextureFormat,
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
            surface_format: DEFAULT_TEXTURE_FORMAT,
        }
    }

    pub fn with_surface_format(&self, surface_format: wgpu::TextureFormat) -> Self {
        Self {
            surface_format,
            ..self.clone()
        }
    }

    pub fn multi(&self, multiplier: f32) -> GraphicsWindowConf<'_> {
        let [x, y] = self.dims;
        GraphicsWindowConf {
            device: self.device,
            dims: [
                (x as f32 * multiplier) as u32,
                (y as f32 * multiplier) as u32,
            ],
            assets_path: GraphicsAssets::Nothing,
            surface_format: self.surface_format,
        }
    }

    pub fn dims(&self) -> [u32; 2] {
        self.dims
    }

    pub fn device(&self) -> &wgpu::Device {
        self.device.device()
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

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }
}
