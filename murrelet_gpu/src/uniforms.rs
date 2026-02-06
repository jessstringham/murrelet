use bytemuck::{Pod, Zeroable};
#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BasicUniform {
    dims: [f32; 4],
    pub more_info: [f32; 4],
    pub more_info_other: [f32; 4],
}

unsafe impl Zeroable for BasicUniform {}
unsafe impl Pod for BasicUniform {}

impl BasicUniform {
    fn empty_4() -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    pub fn from_empty() -> BasicUniform {
        BasicUniform {
            dims: BasicUniform::empty_4(),
            more_info: BasicUniform::empty_4(),
            more_info_other: BasicUniform::empty_4(),
        }
    }

    fn _dims_to_more_info(w: f32, h: f32) -> [f32; 4] {
        [w, h, 1.0 / w, 1.0 / h]
    }

    pub fn from_dims([w, h]: [u32; 2]) -> BasicUniform {
        let w_f32 = w as f32;
        let h_f32 = h as f32;
        let dims = BasicUniform::_dims_to_more_info(w_f32, h_f32);
        BasicUniform {
            dims,
            more_info: BasicUniform::empty_4(),
            more_info_other: BasicUniform::empty_4(),
        }
    }

    pub fn from_dims_and_more([w, h]: [u32; 2], more_info: [f32; 4]) -> BasicUniform {
        let w_f32 = w as f32;
        let h_f32 = h as f32;
        let dims = BasicUniform::_dims_to_more_info(w_f32, h_f32);
        BasicUniform {
            dims,
            more_info,
            more_info_other: BasicUniform::empty_4(),
        }
    }

    pub fn update_more_info(&mut self, more_info: [f32; 4]) {
        self.more_info = more_info
    }

    pub fn update_more_info_other(&mut self, more_info: [f32; 4]) {
        self.more_info_other = more_info
    }

    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    fn uniforms_size(&self) -> u64 {
        std::mem::size_of::<Self>() as wgpu::BufferAddress
    }

    pub fn to_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: self.uniforms_size(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

pub struct UniformsPair {
    pub more_info: [f32; 4],
    pub more_info_other: [f32; 4],
}
impl UniformsPair {
    pub fn new(more_info: [f32; 4], more_info_other: [f32; 4]) -> UniformsPair {
        UniformsPair {
            more_info,
            more_info_other,
        }
    }
}
