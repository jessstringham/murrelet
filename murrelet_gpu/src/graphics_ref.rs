#![allow(dead_code)]
use std::{cell::RefCell, sync::Arc};

use bytemuck::{Pod, Zeroable};
use std::rc::Rc;

#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

// some wgpu things
use wgpu::util::DeviceExt;
use wgpu::TextureDescriptor;

use crate::device_state::*;
use crate::shader_str::VERTEX_SHADER;

#[cfg(not(feature = "nannou"))]
pub const DEFAULT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
#[cfg(feature = "nannou")]
pub const DEFAULT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[cfg(not(feature = "nannou"))]
pub const DEFAULT_LOADED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
#[cfg(feature = "nannou")]
pub const DEFAULT_LOADED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn shader_from_path(device: &wgpu::Device, data: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(data.into()),
    })
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    position: [f32; 2],
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}

pub const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
];

#[derive(Debug, Copy, Clone)]
pub struct ShaderOptions {
    sampler_address_mode_u: wgpu::AddressMode,
    sampler_address_mode_v: wgpu::AddressMode,
    sampler_address_mode_w: wgpu::AddressMode,
    sampler_mag_filter: wgpu::FilterMode,
    sampler_min_filter: wgpu::FilterMode,
    sampler_mipmap_filter: wgpu::FilterMode,
}
impl ShaderOptions {
    pub fn new() -> ShaderOptions {
        ShaderOptions {
            sampler_address_mode_u: wgpu::AddressMode::ClampToBorder,
            sampler_address_mode_v: wgpu::AddressMode::ClampToBorder,
            sampler_address_mode_w: wgpu::AddressMode::ClampToBorder,
            sampler_mag_filter: wgpu::FilterMode::Nearest,
            sampler_min_filter: wgpu::FilterMode::Nearest,
            sampler_mipmap_filter: wgpu::FilterMode::Nearest,
        }
    }

    pub fn new_with_options(
        mag_filter: wgpu::FilterMode,
        address_mode: wgpu::AddressMode,
    ) -> ShaderOptions {
        ShaderOptions {
            sampler_address_mode_u: address_mode,
            sampler_address_mode_v: address_mode,
            sampler_address_mode_w: address_mode,
            sampler_mag_filter: mag_filter,
            sampler_min_filter: wgpu::FilterMode::Nearest,
            sampler_mipmap_filter: wgpu::FilterMode::Nearest,
        }
    }

    fn as_sampler_desc(&self) -> wgpu::SamplerDescriptor {
        wgpu::SamplerDescriptor {
            address_mode_u: self.sampler_address_mode_u,
            address_mode_v: self.sampler_address_mode_v,
            address_mode_w: self.sampler_address_mode_w,
            mag_filter: self.sampler_mag_filter,
            min_filter: self.sampler_min_filter,
            mipmap_filter: self.sampler_mipmap_filter,
            ..Default::default()
        }
    }

    fn with_mag_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.sampler_mag_filter = filter;
        self
    }

    fn with_min_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.sampler_min_filter = filter;
        self
    }

    fn with_mip_map_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.sampler_mipmap_filter = filter;
        self
    }

    fn with_address_mode(mut self, address_mode: wgpu::AddressMode) -> Self {
        self.sampler_address_mode_u = address_mode;
        self.sampler_address_mode_v = address_mode;
        self.sampler_address_mode_w = address_mode;
        self
    }
}
impl Default for ShaderOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Copy, Clone)]
struct TextureCreator {
    format: wgpu::TextureFormat,
}

#[derive(Debug, Copy, Clone)]
pub struct GraphicsCreator {
    first_texture: TextureCreator,
    second_texture: Option<TextureCreator>,
    details: ShaderOptions,
    color_blend: wgpu::BlendComponent,
    dst_texture: TextureCreator,
}
impl Default for GraphicsCreator {
    fn default() -> Self {
        GraphicsCreator {
            first_texture: TextureCreator {
                format: DEFAULT_TEXTURE_FORMAT,
            },
            second_texture: None,
            details: ShaderOptions::new_with_options(
                wgpu::FilterMode::Linear,
                wgpu::AddressMode::ClampToEdge,
            ),
            color_blend: wgpu::BlendComponent::REPLACE,
            dst_texture: TextureCreator {
                format: DEFAULT_TEXTURE_FORMAT,
            },
        }
    }
}
impl GraphicsCreator {
    pub fn with_first_texture_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.first_texture = TextureCreator { format };
        self
    }

    pub fn with_second_texture(mut self) -> Self {
        self.second_texture = Some(TextureCreator {
            format: DEFAULT_TEXTURE_FORMAT,
        });
        self
    }

    pub fn with_second_texture_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.second_texture = Some(TextureCreator { format });
        self
    }

    pub fn with_dst_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.dst_texture = TextureCreator { format };
        self
    }

    pub fn with_mag_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.details = self.details.with_mag_filter(filter);
        self
    }

    pub fn with_min_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.details = self.details.with_min_filter(filter);
        self
    }

    pub fn with_mip_map_filter(mut self, filter: wgpu::FilterMode) -> Self {
        self.details = self.details.with_mip_map_filter(filter);
        self
    }

    pub fn with_color_blend(mut self, blend: wgpu::BlendComponent) -> Self {
        self.color_blend = blend;
        self
    }

    pub fn with_address_mode(mut self, address: wgpu::AddressMode) -> Self {
        self.details = self.details.with_address_mode(address);
        self
    }

    pub fn to_graphics_ref<'a>(
        &self,
        c: &GraphicsWindowConf<'a>,
        name: &str,
        fs_shader: &str,
    ) -> GraphicsRef {
        if self.color_blend != wgpu::BlendComponent::REPLACE
            && self.dst_texture.format == wgpu::TextureFormat::Rgba32Float
        {
            panic!("can't blend with float32 textures");
        }

        GraphicsRef::new(name, c, fs_shader, self)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BasicUniform {
    dims: [f32; 4],
    more_info: [f32; 4],
    more_info_other: [f32; 4],
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

    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    fn uniforms_size(&self) -> u64 {
        std::mem::size_of::<Self>() as wgpu::BufferAddress
    }

    fn to_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: self.uniforms_size(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn copy_to_buffer(
        &self,
        dest: &wgpu::Buffer,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        encoder.copy_buffer_to_buffer(&self.to_buffer(device), 0, dest, 0, self.uniforms_size());
    }
}

pub struct UniformsPair {
    more_info: [f32; 4],
    more_info_other: [f32; 4],
}
impl UniformsPair {
    pub fn new(more_info: [f32; 4], more_info_other: [f32; 4]) -> UniformsPair {
        UniformsPair {
            more_info,
            more_info_other,
        }
    }
}

#[derive(Clone)]
pub struct GraphicsRef {
    pub graphics: Rc<RefCell<Graphics>>,
}

impl GraphicsRef {
    pub fn name(&self) -> String {
        self.graphics.borrow().name.clone()
    }

    pub fn more_info(&self) -> ([f32; 4], [f32; 4]) {
        (
            self.graphics.borrow().uniforms.more_info,
            self.graphics.borrow().uniforms.more_info_other,
        )
    }

    pub fn new_with_src<'a>(
        name: &str,
        c: &GraphicsWindowConf<'a>,
        fs_shader: &str,
        conf: &GraphicsCreator,
        assets: GraphicsAssets,
    ) -> Self {
        let graphics = Graphics::new_mut(
            name.to_string(),
            c,
            fs_shader,
            BasicUniform::from_dims(c.dims()),
            assets,
            conf.clone(),
        );
        Self { graphics }
    }

    pub fn new<'a>(
        name: &str,
        c: &GraphicsWindowConf<'a>,
        fs_shader: &str,
        conf: &GraphicsCreator,
    ) -> Self {
        Self::new_with_src(name, c, fs_shader, conf, GraphicsAssets::Nothing)
    }

    pub fn update_uniforms_other_tuple(
        &self,
        c: &GraphicsWindowConf,

        more_info: ([f32; 4], [f32; 4]),
    ) {
        let mut graphics_rc = self.graphics.borrow_mut();
        graphics_rc.update_uniforms_other_tuple(c, more_info)
    }

    pub fn render_to_view(&self, device: &DeviceState, view: &wgpu::TextureView) {
        self.graphics.borrow_mut().render(device, view)
    }

    pub fn render(&self, device: &DeviceState, other: &GraphicsRef) {
        let view = &other.graphics.borrow_mut().input_texture_view;
        self.graphics.borrow_mut().render(device, view)
    }

    pub fn render_2tex(&self, device_state: &DeviceState, other: &GraphicsRef) {
        let binding = other.graphics.borrow_mut();
        let view = binding.input_texture_view_other.as_ref().unwrap();
        self.graphics.borrow_mut().render(device_state, view)
    }

    pub fn update_uniforms(&self, c: &GraphicsWindowConf, more_info: [f32; 4]) {
        self.graphics.borrow_mut().update_uniforms(c, more_info)
    }

    pub fn update_uniforms_other(
        &self,
        c: &GraphicsWindowConf,
        more_info: [f32; 4],
        more_info_other: [f32; 4],
    ) {
        self.graphics
            .borrow_mut()
            .update_uniforms_other(c, more_info, more_info_other)
    }

    pub fn render_to_texture(&self, device_state: &DeviceState, texture: &wgpu::TextureView) {
        self.graphics.borrow_mut().render(device_state, texture)
    }

    pub fn input_texture_descriptor(&self) -> TextureDescriptor<'static> {
        self.graphics.borrow().texture_and_desc.desc.clone()
    }

    pub fn render_with_custom_bind_group(
        &self,
        device_state: &DeviceState,
        output_texture_view: &wgpu::TextureView,
        bind_group: &wgpu::BindGroup,
    ) {
        self.graphics.borrow().render_with_custom_bind_group(
            device_state,
            output_texture_view,
            bind_group,
        )
    }
}

// for now this is just so nannou can create textures...
#[derive(Debug)]
pub struct TextureAndDesc {
    pub texture: Arc<wgpu::Texture>,
    pub desc: wgpu::TextureDescriptor<'static>,
}

// represents things needed to create a single texture... it's a bit of a mess
pub struct Graphics {
    name: String,
    conf: GraphicsCreator,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    pub uniforms: BasicUniform,
    pub uniforms_buffer: wgpu::Buffer, // used internally
    pub input_texture_view: wgpu::TextureView,
    pub input_texture_view_other: Option<wgpu::TextureView>,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    // i guess need this to create nannou texture
    pub texture_and_desc: TextureAndDesc,
    pub other_texture_and_desc: Option<TextureAndDesc>,
}

impl Graphics {
    pub fn update_uniforms(&mut self, c: &GraphicsWindowConf, more_info: [f32; 4]) {
        let queue = &c.device.queue();
        self.uniforms.more_info = more_info;

        // println!("{:?}", self.uniform.more_info);
        queue.write_buffer(&self.uniforms_buffer, 0, self.uniforms.as_bytes());
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
        queue.write_buffer(&self.uniforms_buffer, 0, self.uniforms.as_bytes());
    }

    pub fn update_uniforms_other_tuple(
        &mut self,
        c: &GraphicsWindowConf,

        more_info: ([f32; 4], [f32; 4]),
    ) {
        let (more_info, more_info_other) = more_info;
        self.update_uniforms_other(c, more_info, more_info_other)
    }

    pub fn update_uniforms_pair(&mut self, c: &GraphicsWindowConf, more_info: UniformsPair) {
        let UniformsPair {
            more_info,
            more_info_other,
        } = more_info;
        self.update_uniforms_other(c, more_info, more_info_other)
    }

    // create a texture
    pub fn texture(
        dim: [u32; 2],
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> TextureAndDesc {
        let desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: dim[0],
                height: dim[1],
                depth_or_array_layers: 1,
            },
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            label: None,
            view_formats: &[],
        };

        TextureAndDesc {
            texture: Arc::new(device.create_texture(&desc)),
            desc,
        }
    }

    fn _bind_group_layout(
        device: &wgpu::Device,
        has_second_texture: bool,
        multisampled: bool,
    ) -> wgpu::BindGroupLayout {
        let mut bind_group_offset = 0;

        let mut bind_group_layout_entries = Vec::new();
        bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: 0 as u32, // needs to line up with @group(0) @binding(1)
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled,
            },
            count: None,
        });

        if has_second_texture {
            bind_group_offset += 1;
            bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 as u32, // needs to line up with @group(0) @binding(0)
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled,
                },
                count: None,
            });
        }

        // next is the sampler
        bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: bind_group_offset + 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });

        // and finally the uniforms
        bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: bind_group_offset + 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &bind_group_layout_entries,
        })
    }

    fn _sampler(device: &wgpu::Device, details: ShaderOptions) -> wgpu::Sampler {
        let sampler_desc = details.as_sampler_desc();
        let sampler = device.create_sampler(&sampler_desc);
        println!("sampler: {:?}, {:?}", sampler, sampler_desc);
        sampler
    }

    fn _bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        input_texture_view: &wgpu::TextureView,
        input_texture_view_other: &Option<wgpu::TextureView>,
        initial_uniform_buffer: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let mut entries = Vec::new();

        let mut binding_offset = 0;

        entries.push(wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&input_texture_view),
        });
        if let Some(texture_view_other) = input_texture_view_other {
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view_other),
            });
            binding_offset += 1;
        }

        // next is the sampler
        entries.push(wgpu::BindGroupEntry {
            binding: binding_offset + 1,
            resource: wgpu::BindingResource::Sampler(&sampler),
        });

        entries.push(wgpu::BindGroupEntry {
            binding: binding_offset + 2,
            resource: initial_uniform_buffer.as_entire_binding(),
        });

        let bf: wgpu::BindGroupDescriptor = wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &entries,
        };

        device.create_bind_group(&bf)
    }

    fn _render_pipeline(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        vs_mod: &wgpu::ShaderModule,
        fs_mod: &wgpu::ShaderModule,
        dst_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let pipeline_layout = Graphics::_pipeline_layout(device, bind_group_layout);

        let vertex_buffer_layouts_attributes = wgpu::vertex_attr_array![0 => Float32x2];

        let vertex_buffer_layouts = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_buffer_layouts_attributes,
        };

        let color_state = vec![Some(wgpu::ColorTargetState {
            format: dst_format,
            blend: Some(wgpu::BlendState::REPLACE), //None,
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let rp_desc = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_mod,
                entry_point: "main",
                buffers: &[vertex_buffer_layouts],
                #[cfg(not(feature = "nannou"))]
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fs_mod,
                entry_point: "main",
                targets: &color_state,
                #[cfg(not(feature = "nannou"))]
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            // cache: None,
        };

        device.create_render_pipeline(&rp_desc)
    }

    fn _vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&VERTICES[..]),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    fn _pipeline_layout(
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        let desc = wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[],
        };
        device.create_pipeline_layout(&desc)
    }

    pub fn new_mut(
        name: String,
        c: &GraphicsWindowConf,
        fs_shader_data: &str,
        initial_uniform: BasicUniform,
        texture_src_path: GraphicsAssets,
        conf: GraphicsCreator,
    ) -> Rc<RefCell<Self>> {
        // todo, i used to have code here to check the conf's destination texture was okay

        let g = Graphics::new(
            name,
            c,
            fs_shader_data,
            initial_uniform,
            texture_src_path,
            conf,
        );

        Rc::new(RefCell::new(g))
    }

    pub fn new<'a>(
        name: String,
        c: &GraphicsWindowConf<'a>,
        fs_shader_data: &str,
        initial_uniform: BasicUniform,
        texture_src_path: GraphicsAssets,
        conf: GraphicsCreator,
    ) -> Self {
        let has_second_texture = conf.second_texture.is_some();
        let details = conf.details;
        let first_format = conf.first_texture.format;
        let second_format = conf.second_texture.map(|x| x.format);
        let dst_format = conf.dst_texture.format;

        let device = c.device.device();
        // todo, figure out msaa samples
        // let msaa_samples = 1;

        let vs_mod = shader_from_path(device, VERTEX_SHADER);
        let fs_mod = shader_from_path(device, fs_shader_data);

        // make a bind group layout

        let first_texture_format = texture_src_path.to_format(first_format);
        let texture_and_desc = Graphics::texture(c.dims, device, first_texture_format);
        let input_texture = &texture_and_desc.texture;
        println!("FIRST input_texture {:?}", input_texture);

        // maybe load the image source if we have one
        texture_src_path.maybe_load_texture(c.device, input_texture);

        let input_texture_view = input_texture.create_view(&Default::default());

        println!("input {:?}", input_texture_view);
        let (input_texture_view_other, other_texture_and_desc) = if has_second_texture {
            let other_texture = Graphics::texture(c.dims(), device, second_format.unwrap());
            println!("other texture view {:?}", &other_texture.texture);
            (
                Some(other_texture.texture.create_view(&Default::default())),
                Some(other_texture),
            )
        } else {
            (None, None)
        };
        println!("other input {:?}", input_texture_view_other);

        let sampler = Graphics::_sampler(device, details);
        let bind_group_layout = Graphics::_bind_group_layout(device, has_second_texture, false);

        let initial_uniform_buffer = initial_uniform.to_buffer(device);

        let render_pipeline =
            Graphics::_render_pipeline(device, &bind_group_layout, &vs_mod, &fs_mod, dst_format);

        let bind_group = Graphics::_bind_group(
            device,
            &bind_group_layout,
            &input_texture_view,
            &input_texture_view_other,
            &initial_uniform_buffer,
            &sampler,
        );

        println!("bind_group {:?}", bind_group);

        Self {
            name,
            conf,
            render_pipeline,
            bind_group,
            uniforms: initial_uniform,
            uniforms_buffer: initial_uniform_buffer,
            vertex_buffer: Graphics::_vertex_buffer(device),
            input_texture_view,
            input_texture_view_other,
            // things i might need to create custom bind groups later
            sampler,
            bind_group_layout,
            // things needed mostly for nannou right now..
            texture_and_desc,
            other_texture_and_desc,
        }
    }

    pub fn make_new_custom_bind_group(
        &self,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        println!("making custom {:?} {:?}", texture_view, self.sampler);

        Graphics::_bind_group(
            device,
            &self.bind_group_layout,
            texture_view,
            &self.input_texture_view_other, // i don't know what to do with this, leave it None or let there be one..
            &self.uniforms_buffer,
            &self.sampler,
        )
    }

    pub fn render_with_custom_bind_group(
        &self,
        device_state: &DeviceState,
        output_texture_view: &wgpu::TextureView,
        bind_group: &wgpu::BindGroup,
    ) {
        let mut encoder = device_state
            .device()
            .create_command_encoder(&Default::default());

        {
            let render_pass_desc = wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        #[cfg(not(feature = "nannou"))]
                        store: wgpu::StoreOp::Store,
                        #[cfg(feature = "nannou")]
                        store: true,
                    },
                })],
                #[cfg(not(feature = "nannou"))]
                occlusion_query_set: None,
                #[cfg(not(feature = "nannou"))]
                timestamp_writes: None,
                depth_stencil_attachment: None,
            };

            let mut rpass = encoder.begin_render_pass(&render_pass_desc);
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.draw(0..VERTICES.len() as u32, 0..1);
            drop(rpass);
        }

        device_state.queue().submit(Some(encoder.finish()));
    }

    pub fn render(&self, device: &DeviceState, output_texture_view: &wgpu::TextureView) {
        self.render_with_custom_bind_group(device, output_texture_view, &self.bind_group)
    }
}

pub fn quick_texture(dims: [u32; 2], device: &wgpu::Device) -> TextureAndDesc {
    Graphics::texture(dims, device, DEFAULT_TEXTURE_FORMAT)
}
