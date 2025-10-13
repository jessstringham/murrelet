use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};

use bytemuck::Pod;
use glam::Vec2;
use itertools::Itertools;
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
impl ComputeBindings {
    fn update_csr_and_data<T: Pod>(&mut self, c: &GraphicsWindowConf, csr: CSR, data: Vec<T>) {
        let device = c.device();

        let mut offsets = csr.offsets;
        if offsets.is_empty() {
            offsets = vec![0; 2]
        };
        let mut indices = csr.indices;
        if indices.is_empty() {
            indices = vec![0; 2]
        };

        // let queue = c.queue();
        // queue.write_buffer(&self.cell_offsets, 0, bytemuck::cast_slice(&offsets));
        // queue.write_buffer(&self.cell_indices, 0, bytemuck::cast_slice(&indices));
        // queue.write_buffer(&self.input, 0, bytemuck::cast_slice(&data));

        self.input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        self.cell_indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        self.cell_offsets = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&offsets),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
    }
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

    fn from_data<T: ToAABB>(nx: u32, ny: u32, data: &[T]) -> Self {
        let cell_count = (nx * ny) as usize;
        let mut orig_cells = vec![vec![]; cell_count];

        // helpers
        // let clamp01 = |v: f32| v.max(0.0).min(1.0);
        let cell_id = |x_i: u32, y_i: u32| -> usize { (y_i * nx + x_i) as usize };
        let ix_clamp = |x: i32| -> u32 { x.max(0).min(nx as i32 - 1) as u32 };
        let iy_clamp = |y: i32| -> u32 { y.max(0).min(ny as i32 - 1) as u32 };

        for (idx, d) in data.iter().enumerate() {
            let bounds = d.to_aabb();

            // Convert to cell-range (inclusive)
            let ix0 = ix_clamp((bounds.min.x * nx as f32).floor() as i32);
            let ix1 = ix_clamp((bounds.max.x * nx as f32).floor() as i32);
            let iy0 = iy_clamp((bounds.min.y * ny as f32).floor() as i32);
            let iy1 = iy_clamp((bounds.max.y * ny as f32).floor() as i32);

            for iy in iy0..=iy1 {
                for ix in ix0..=ix1 {
                    orig_cells[cell_id(ix, iy)].push(idx as u32);
                }
            }
        }

        // now go through the cells and append the neighboring cells!
        let mut cells = vec![vec![]; cell_count];

        for x_i in 0..nx {
            for y_i in 0..ny {
                let mut cell_val = HashSet::new();

                for offset_x in -1..=1 {
                    for offset_y in -1..=1 {
                        let xii = ix_clamp(x_i as i32 + offset_x);
                        let yii = iy_clamp(y_i as i32 + offset_y);

                        for c in &orig_cells[cell_id(xii, yii)] {
                            cell_val.insert(*c);
                        }
                    }
                }
                cells[cell_id(x_i, y_i)] = cell_val.into_iter().collect_vec();
            }
        }

        CSRData { cells }
    }
}

pub struct AABB {
    pub min: Vec2,
    pub max: Vec2,
}

pub trait ToAABB {
    fn to_aabb(&self) -> AABB;
}

// like Graphics, what's needed to create a compute pipeline
pub struct ComputeGraphicsToTexture {
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
    // data: Vec<T>,
    dims: [u32; 2],
    // csr: CSR,
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
impl ComputeGraphicsToTexture {
    fn sync_data<T: Pod + ToAABB + Clone>(
        &mut self,
        c: &GraphicsWindowConf,
        nx: u32,
        ny: u32,
        data: &[T],
    ) {
        // the data should already be scaled 0.0 to 1.0

        // make sure we use the same vars on both sides
        self.update_uniforms_other(c, [nx as f32, ny as f32, 0.0, 0.0], [0.0; 4]);

        // build csr
        let csr = CSRData::from_data(nx, ny, data).for_buffers();

        let data = data.to_vec();

        self.buffers.update_csr_and_data(c, csr, data);
    }

    pub fn init<'a, T: Pod + ToAABB + Clone>(
        name: String,
        c: &GraphicsWindowConf<'a>,
        compute_shader: &str,
        data: Vec<T>,
    ) -> ComputeGraphicsToTextureRef {
        ComputeGraphicsToTextureRef::new(Rc::new(RefCell::new(Self::new(
            name,
            c,
            compute_shader,
            BasicUniform::from_dims(c.dims),
            CSR::empty(),
            data,
        ))))
    }

    pub fn new<'a, T: Pod + ToAABB + Clone>(
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
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let cell_indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&csr.indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let cell_offsets_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&csr.offsets),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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
                // 21: CSR offsets
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
                        format: wgpu::TextureFormat::Rgba16Float,
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
            // data,
            pipeline,
            buffers,
            uniforms: initial_uniform,
            bind_group_layout,
            texture,
            dims,
            // csr: CSR::empty(),
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
pub struct ComputeGraphicsToTextureRef {
    pub graphics: Rc<RefCell<ComputeGraphicsToTexture>>,
}
impl ComputeGraphicsToTextureRef {
    fn new(graphics: Rc<RefCell<ComputeGraphicsToTexture>>) -> Self {
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

    pub fn sync_data<T: Pod + ToAABB + Clone>(
        &self,
        c: &GraphicsWindowConf,
        nx: u32,
        ny: u32,
        segments: &[T],
    ) {
        if !segments.is_empty() {
            self.graphics.borrow_mut().sync_data(c, nx, ny, segments)
        } else {
            println!("segments is empty, not doing anything");
        }
    }
}
