use bytemuck::{Pod, Zeroable};
use itertools::Itertools;
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BonusUniform {
    dims: [f32; 4],
    pub more_info_secret: [f32; 4],
    pub more_info1: [f32; 4],
    pub more_info2: [f32; 4],
    pub more_info3: [f32; 4],
    pub more_info4: [f32; 4],
}

impl BonusUniform {
    fn empty_4() -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    pub fn from_empty() -> BonusUniform {
        BonusUniform {
            dims: BonusUniform::empty_4(),
            more_info_secret: BonusUniform::empty_4(),
            more_info1: BonusUniform::empty_4(),
            more_info2: BonusUniform::empty_4(),
            more_info3: BonusUniform::empty_4(),
            more_info4: BonusUniform::empty_4(),
        }
    }

    fn _dims_to_more_info(w: f32, h: f32) -> [f32; 4] {
        [w, h, 1.0 / w, 1.0 / h]
    }

    pub fn from_dims([w, h]: [u32; 2]) -> BonusUniform {
        let w_f32 = w as f32;
        let h_f32 = h as f32;
        let dims = BonusUniform::_dims_to_more_info(w_f32, h_f32);
        BonusUniform {
            dims,
            more_info_secret: BonusUniform::empty_4(),
            more_info1: BonusUniform::empty_4(),
            more_info2: BonusUniform::empty_4(),
            more_info3: BonusUniform::empty_4(),
            more_info4: BonusUniform::empty_4(),
        }
    }


    pub fn update_up_to_16(&mut self, more_info: &[f32]) {
        if more_info.len() > 16 {
            println!("more info is longer than 16, only will store first 16");
        }

        // eh
        let mut v = vec![];
        for i in 0..16 {
            v.push(more_info.get(i).copied().unwrap_or_default());
        }
        // now we know we have 16 things

        self.more_info1 = v[0..4].try_into().unwrap();
        self.more_info2 = v[4..8].try_into().unwrap();
        self.more_info3 = v[8..12].try_into().unwrap();
        self.more_info4 = v[12..16].try_into().unwrap();


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

unsafe impl Zeroable for BonusUniform {}
unsafe impl Pod for BonusUniform {}

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
