// stores info about the window..

use crate::{
    device_state::{DeviceState, GraphicsAssets},
    graphics_ref::DEFAULT_TEXTURE_FORMAT,
};

#[derive(Clone, Debug)]
pub struct GraphicsWindowConf<'a> {
    pub device_state: &'a DeviceState<'a>,
    pub dims: [u32; 2],
    pub assets_path: GraphicsAssets,
    pub display_format: wgpu::TextureFormat,
}
impl<'a> GraphicsWindowConf<'a> {
    pub fn new(
        device_state: &'a DeviceState,
        dims: [u32; 2],
        assets_path: GraphicsAssets,
        display_format: wgpu::TextureFormat,
    ) -> GraphicsWindowConf<'a> {
        GraphicsWindowConf {
            device_state,
            dims,
            assets_path,
            display_format,
        }
    }

    pub fn offscreen(
        device_state: &'a DeviceState,
        dims: [u32; 2],
        assets_path: GraphicsAssets,
    ) -> GraphicsWindowConf<'a> {
        GraphicsWindowConf {
            device_state,
            dims,
            assets_path,
            display_format: DEFAULT_TEXTURE_FORMAT,
        }
    }

    pub fn with_surface_format(&self, surface_format: wgpu::TextureFormat) -> Self {
        Self {
            display_format: surface_format,
            ..self.clone()
        }
    }

    pub fn multi(&self, multiplier: f32) -> GraphicsWindowConf<'_> {
        let [x, y] = self.dims;
        GraphicsWindowConf {
            dims: [
                (x as f32 * multiplier) as u32,
                (y as f32 * multiplier) as u32,
            ],
            ..self.clone()
        }
    }

    pub fn dims(&self) -> [u32; 2] {
        self.dims
    }

    pub fn device_state(&'a self) -> &'a DeviceState<'a> {
        self.device_state
    }

    pub fn device(&self) -> &wgpu::Device {
        self.device_state.device()
    }

    pub fn with_dims(&self, dims: [u32; 2]) -> Self {
        Self {
            dims,
            ..self.clone()
        }
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.device_state.queue()
    }

    pub fn display_format(&self) -> wgpu::TextureFormat {
        self.display_format
    }
}
