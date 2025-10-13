use std::{cell::RefCell, rc::Rc, sync::Arc};

use bytemuck::Pod;
#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

use wgpu::util::DeviceExt;

use crate::{
    device_state::{DeviceState, DeviceStateForRender},
    graphics_ref::{shader_from_path, GraphicsRef, TextureAndDesc, DEFAULT_TEXTURE_FORMAT},
    uniforms::BasicUniform,
    window::GraphicsWindowConf,
};

struct ComputeBindings {
    input: wgpu::Buffer,
    cell_offsets: wgpu::Buffer,
    cell_indices: wgpu::Buffer,
    uniforms: wgpu::Buffer,
}

pub struct CSR {
    offsets: Vec<u32>, // size is N + 1, contains the start/end of each group! fence post
    indices: Vec<u32>,
}
impl CSR {
    fn empty() -> CSR {
        Self {
            offsets: vec![0, 0], // need to put something...
            indices: vec![0],
        }
    }
}

pub struct CSRData {
    cells: Vec<Vec<u32>>,
}
impl CSRData {
    pub fn new(cells: Vec<Vec<u32>>) -> Self {
        Self { cells }
    }

    fn for_buffers(self) -> CSR {
        let mut offsets = Vec::with_capacity(self.cells.len() + 1);
        let mut indices = Vec::new();
        offsets.push(0);
        for v in self.cells {
            indices.extend_from_slice(&v);
            offsets.push(indices.len() as u32);
        }
        CSR { offsets, indices }
    }
}

// like Graphics, what's needed to create a compute pipeline
pub struct ComputeGraphicsToTexture<T> {
    name: String,
    // conf: GraphicsCreator,
    // bind_group: wgpu::BindGroup,
    // vertex_buffers: VertexBuffers,
    // render_pipeline: wgpu::RenderPipeline,
    pub uniforms: BasicUniform,
    pub buffers: ComputeBindings,
    texture: TextureAndDesc,

    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    data: Vec<T>,
    dims: [u32; 2],
    // used internally
    // pub input_texture_view: wgpu::TextureView,
    // pub input_texture_view_other: Option<wgpu::TextureView>,
    // sampler: wgpu::Sampler,
    // bind_group_layout: wgpu::BindGroupLayout,
    // // i guess need this to create nannou texture
    // pub texture_and_desc: TextureAndDesc,
    // pub other_texture_and_desc: Option<TextureAndDesc>,
    // textures_for_3d: Option<TextureFor3d>,
}
impl<T: Pod> ComputeGraphicsToTexture<T> {
    pub fn init<'a>(
        name: String,
        c: &GraphicsWindowConf<'a>,
        compute_shader: &str,
    ) -> ComputeGraphicsToTextureRef<T> {
        ComputeGraphicsToTextureRef::new(Rc::new(RefCell::new(Self::new(
            name,
            c,
            compute_shader,
            BasicUniform::from_empty(),
            CSR::empty(),
            vec![],
        ))))
    }

    pub fn new<'a>(
        name: String,
        c: &GraphicsWindowConf<'a>,
        shader_data: &str,
        initial_uniform: BasicUniform,
        csr: CSR, // helps limit what data you need to check per cell
        data: Vec<T>,
    ) -> Self {
        let device: &wgpu::Device = c.device.device();

        let input_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let cell_indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&csr.indices),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let cell_offsets_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&csr.offsets),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let uniforms_buffer = initial_uniform.to_buffer(device);

        // now create the layout!
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute bind group layout"),
            entries: &[
                // 0: input
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: CSR offsets
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: CSR indices
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // and the output texture
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm, // match your texture/WGSL
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                // // add back for cpu output!
                // wgpu::BindGroupLayoutEntry {
                //     binding: 1,
                //     visibility: wgpu::ShaderStages::COMPUTE,
                //     ty: wgpu::BindingType::Buffer {
                //         ty: wgpu::BufferBindingType::Storage { read_only: false },
                //         has_dynamic_offset: false,
                //         min_binding_size: None,
                //     },
                //     count: None,
                // },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // let out_texture = output_texture
        //     .graphics()
        //     .graphics
        //     .borrow()
        //     .texture_and_desc
        //     .texture
        //     .create_view(&Default::default());

        let desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: c.dims[0],
                height: c.dims[1],
                depth_or_array_layers: 1,
            },
            format: DEFAULT_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            label: None,
            view_formats: &[],
        };

        let texture = TextureAndDesc {
            texture: Arc::new(device.create_texture(&desc)),
            desc,
        };

        let buffers = ComputeBindings {
            uniforms: uniforms_buffer,
            input: input_data_buffer,
            cell_offsets: cell_offsets_buffer,
            cell_indices: cell_indices_buffer,
        };

        // now bind the buffers!
        // let bind_group = Self::bind_group(
        //     device,
        //     &bind_group_layout,
        //     &buffers,
        //     &texture.default_view(),
        // );

        let shader = shader_from_path(device, shader_data);

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            #[cfg(not(feature = "nannou"))]
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            #[cfg(not(feature = "nannou"))]
            cache: None,
        });

        let dims = c.dims;

        Self {
            name,
            data,
            pipeline,
            buffers,
            uniforms: initial_uniform,
            bind_group_layout,
            texture,
            dims,
        }
    }

    // from https://github.com/gfx-rs/wgpu/blob/1cbebdcffe64c05e8ed14db7331333425f6feb65/examples/standalone/01_hello_compute/src/main.rs

    pub fn render(&self, d: &DeviceState, output_texture_view: &wgpu::TextureView) {
        // first compute
        let device = d.device();
        let queue = d.queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compute encoder"),
        });

        {
            // update our output texture
            let bind_group = Self::bind_group(
                device,
                &self.bind_group_layout,
                &self.buffers,
                output_texture_view,
            );

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute pass"),
                #[cfg(not(feature = "nannou"))]
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // dispatch over image size (stored in uniforms)
            let [w, h] = self.dims; // ideally could get this from the texture...
            const WGX: u32 = 8;
            const WGY: u32 = 8;
            let gx = (w + WGX - 1) / WGX;
            let gy = (h + WGY - 1) / WGY;

            pass.dispatch_workgroups(gx, gy, 1);
        } // drop(pass)

        queue.submit(Some(encoder.finish()));

        // now texture has teh output
    }

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
        queue.write_buffer(&self.buffers.uniforms, 0, self.uniforms.as_bytes());
    }

    fn bind_group(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        buffers: &ComputeBindings,
        texture: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.cell_offsets.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.cell_indices.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
            ],
        })
    }
}

#[derive(Clone)]
pub struct ComputeGraphicsToTextureRef<T> {
    pub graphics: Rc<RefCell<ComputeGraphicsToTexture<T>>>,
}
impl<T: Pod> ComputeGraphicsToTextureRef<T> {
    fn new(graphics: Rc<RefCell<ComputeGraphicsToTexture<T>>>) -> Self {
        Self { graphics }
    }

    pub fn name(&self) -> String {
        self.graphics.borrow().name.clone()
    }

    pub fn render(&self, device_state_for_render: &DeviceStateForRender, other: &GraphicsRef) {
        let view = &other.graphics.borrow_mut().input_texture_view;
        self.graphics
            .borrow()
            .render(device_state_for_render.device_state(), view)
    }
}
