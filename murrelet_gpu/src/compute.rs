use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

use crate::{uniforms::BasicUniform, window::GraphicsWindowConf};

// like Graphics, what's needed to create a compute pipeline
pub struct ComputeGraphics {
    name: String,
    // conf: GraphicsCreator,
    // bind_group: wgpu::BindGroup,
    // vertex_buffers: VertexBuffers,
    // render_pipeline: wgpu::RenderPipeline,
    pub uniforms: BasicUniform,
    pub uniforms_buffer: wgpu::Buffer, // used internally
    // pub input_texture_view: wgpu::TextureView,
    // pub input_texture_view_other: Option<wgpu::TextureView>,
    // sampler: wgpu::Sampler,
    // bind_group_layout: wgpu::BindGroupLayout,
    // // i guess need this to create nannou texture
    // pub texture_and_desc: TextureAndDesc,
    // pub other_texture_and_desc: Option<TextureAndDesc>,
    // textures_for_3d: Option<TextureFor3d>,
}
impl ComputeGraphics {
    pub fn update_uniforms_other(
        &mut self,
        c: &GraphicsWindowConf,
        more_info: [f32; 4],
        more_info_other: [f32; 4],
    ) {
        let queue = &c.device.queue();
        self.uniforms.more_info = more_info;
        self.uniforms.more_info_other = more_info_other;

        // println!("{:?}", self.uniform.more_info);
        queue.write_buffer(&self.uniforms_buffer, 0, self.uniforms.as_bytes());
    }
}

#[derive(Clone)]
pub struct ComputeShaderRef {
    pub graphics: Rc<RefCell<ComputeGraphics>>,
}
