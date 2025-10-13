#![allow(dead_code)]
use std::{cell::RefCell, sync::Arc};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use itertools::Itertools;
use murrelet_common::triangulate::{Triangulate, VertexSimple};
use std::rc::Rc;

#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

// some wgpu things
use wgpu::util::DeviceExt;
use wgpu::TextureDescriptor;

use crate::device_state::*;
use crate::gpu_livecode::{ControlGraphics, ControlGraphicsRef};
use crate::shader_str::{VERTEX_SHADER, VERTEX_SHADER_3D};
use crate::uniforms::{BasicUniform, UniformsPair};
use crate::window::GraphicsWindowConf;

#[cfg(not(feature = "nannou"))]
pub const DEFAULT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
#[cfg(feature = "nannou")]
pub const DEFAULT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[cfg(not(feature = "nannou"))]
pub const DEFAULT_LOADED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
#[cfg(feature = "nannou")]
pub const DEFAULT_LOADED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub fn shader_from_path(device: &wgpu::Device, data: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(data.into()),
    })
}

// for each vertex, this is what we'll pass in

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    face_pos: [f32; 2],
}

impl Vertex {
    pub fn new(position: [f32; 3], normal: [f32; 3], face_pos: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            face_pos,
        }
    }
    pub fn pos(&self) -> [f32; 3] {
        self.position
    }

    pub fn pos_vec3(&self) -> Vec3 {
        glam::vec3(self.position[0], self.position[1], self.position[2])
    }

    pub fn from_simple(vs: &VertexSimple) -> Self {
        Self {
            position: vs.position,
            normal: vs.normal,
            face_pos: vs.face_pos,
        }
    }
}

unsafe impl Zeroable for Vertex {}
unsafe impl Pod for Vertex {}

// in the default vertex shader, z is dropped
pub const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, 1.0, 0.0],
        normal: [0.0, 0.0, 0.0],
        face_pos: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0],
        normal: [0.0, 0.0, 0.0],
        face_pos: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        normal: [0.0, 0.0, 0.0],
        face_pos: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0],
        normal: [0.0, 0.0, 0.0],
        face_pos: [1.0, 0.0],
    },
];

// when you want to use vertices for real!!
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexUniforms {
    view_proj: [[f32; 4]; 4],  // 4x4 matrix
    light_proj: [[f32; 4]; 4], // 4x4 matrix, to make the view of the light
}
impl VertexUniforms {
    fn from_mat4(view: Mat4, light: Mat4) -> Self {
        Self {
            view_proj: view.to_cols_array_2d(),
            light_proj: light.to_cols_array_2d(),
        }
    }

    fn identity() -> VertexUniforms {
        Self::from_mat4(Mat4::IDENTITY, Mat4::IDENTITY)
    }

    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    fn uniforms_size(&self) -> u64 {
        std::mem::size_of::<Self>() as wgpu::BufferAddress
    }

    fn write_buffer(&self, dest: &wgpu::Buffer, queue: &wgpu::Queue) {
        queue.write_buffer(dest, 0, self.as_bytes());
    }

    fn to_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view and light proj buffer for 3d vertex shader"),
            size: self.uniforms_size(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

pub struct Scene {
    view: VertexUniforms, // update this as needed
}

// this is the conf that you'll interface with
// #[derive(Debug, Clone)]
// pub struct Triangulate {
//     vertices: Vec<Vertex>,
//     order: Vec<u32>,
// }

// impl Triangulate {
//     pub fn new() -> Self {
//         Triangulate {
//             vertices: vec![],
//             order: vec![],
//         }
//     }

//     pub fn vertices(&self) -> &[Vertex] {
//         &self.vertices
//     }

//     pub fn add_vertex(&mut self, v: [f32; 3], n: [f32; 3], face_pos: [f32; 2]) -> u32 {
//         let vv = Vertex::new(v, n, face_pos);
//         self.vertices.push(vv);
//         (self.vertices.len() - 1) as u32
//     }

//     // alternatively can add vertices and then add teh vec
//     pub fn add_rect(&mut self, v: &[Vec3; 4], flip: bool) {
//         let edge1 = v[0] - v[1];
//         let edge2 = v[3] - v[1];
//         let normal = edge1.cross(edge2).normalize().to_array();

//         let v0 = self.add_vertex(v[0].to_array(), normal, [1.0, 0.0]);
//         let v1 = self.add_vertex(v[1].to_array(), normal, [0.0, 0.0]);
//         let v2 = self.add_vertex(v[2].to_array(), normal, [1.0, 1.0]);
//         let v3 = self.add_vertex(v[3].to_array(), normal, [0.0, 1.0]);

//         if !flip {
//             self.order.extend([v0, v2, v1, v1, v2, v3])
//         } else {
//             self.order.extend([v0, v1, v2, v1, v3, v2])
//         }
//     }

//     pub fn set_order(&mut self, u: Vec<u32>) {
//         self.order = u;
//     }

//     fn order(&self) -> &[u32] {
//         &self.order
//     }

//     pub fn indices(&self) -> &[u32] {
//         &self.order
//     }

//     pub fn add_order(&mut self, collect: &[u32]) {
//         self.order.extend_from_slice(collect);
//     }
// }

// this is the conf that you'll interface with
#[derive(Debug, Clone)]
pub struct InputVertexConf {
    is_3d: bool, // todo, maybe can simplify now that i have this, e.g. vs_mod
    vs_mod: &'static str,
    view: VertexUniforms,
    topology: wgpu::PrimitiveTopology,
    vertices: Vec<Vertex>,
    order: Vec<u32>,
}

impl InputVertexConf {
    pub fn buffer_slice(&self) -> &[u32] {
        self.order.as_slice()
    }

    pub fn from_triangulate_2d(t: &Triangulate) -> Self {
        let mut c = Self::default();
        c.vertices = t
            .vertices
            .iter()
            .map(|x| Vertex::from_simple(x))
            .collect_vec();
        c.order = t.order.clone();
        c
    }

    pub fn from_triangulate(t: &Triangulate) -> Self {
        let mut c = Self::default();
        c.is_3d = true;
        c.vs_mod = VERTEX_SHADER_3D;
        c.vertices = t
            .vertices
            .iter()
            .map(|x| Vertex::from_simple(x))
            .collect_vec();
        c.order = t.order.clone();
        c
    }

    pub fn set_view(mut self, view: Mat4, light: Mat4) -> Self {
        self.view = VertexUniforms::from_mat4(view, light);
        self
    }

    pub fn vs_mod(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        shader_from_path(device, self.vs_mod)
    }

    pub fn shadow_vs_mod(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        shader_from_path(
            device,
            "
@group(0) @binding(0) var<uniform> light_proj_view: mat4x4<f32>;

@vertex
fn main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return light_proj_view * vec4<f32>(position, 1.0);
}",
        )
    }

    pub fn with_custom_vertices(mut self, tri: &Triangulate) -> Self {
        self.vertices = tri
            .vertices
            .iter()
            .map(|x| Vertex::from_simple(x))
            .collect_vec();
        self.topology = wgpu::PrimitiveTopology::TriangleList;
        self.order = tri.order.clone();
        self
    }

    pub fn indices(&self) -> u32 {
        self.order.len() as u32
    }

    pub fn default() -> Self {
        Self {
            vs_mod: VERTEX_SHADER,
            view: VertexUniforms::identity(),
            topology: wgpu::PrimitiveTopology::TriangleList,
            vertices: VERTICES.to_vec(),
            order: vec![0, 1, 2, 1, 3, 2],
            is_3d: false,
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct GraphicsCreator {
    first_texture: TextureCreator,
    second_texture: Option<TextureCreator>,
    details: ShaderOptions,
    color_blend: wgpu::BlendComponent,
    dst_texture: TextureCreator,
    input_vertex: InputVertexConf, // defaults to the square
    blend_state: wgpu::BlendState,
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
            input_vertex: InputVertexConf::default(),
            blend_state: wgpu::BlendState::REPLACE,
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

    pub fn with_custom_triangle(mut self, t: &Triangulate, is_3d: bool) -> Self {
        if is_3d {
            self.input_vertex = InputVertexConf::from_triangulate(t);
        } else {
            self.input_vertex = InputVertexConf::from_triangulate_2d(t);
        }
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

    pub fn with_blend_state(mut self, blend_state: wgpu::BlendState) -> Self {
        self.blend_state = blend_state;
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

    fn is_3d(&self) -> bool {
        self.input_vertex.is_3d
    }

    pub fn blend_state(&self) -> wgpu::BlendState {
        self.blend_state
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
        println!("name {:?}", name);
        let graphics = Graphics::new_mut(
            name.to_string(),
            c,
            fs_shader,
            BasicUniform::from_dims(c.dims()),
            assets,
            conf.clone(),
        );
        println!("done name {:?}", name);
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

    pub fn update_view(&self, c: &GraphicsWindowConf, view: Mat4, light: Mat4) {
        self.graphics.borrow_mut().update_view(c, view, light);
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

    pub fn with_control_graphics<T>(
        &self,
        label: &'static str,
        control_graphic_fn: Arc<impl Fn(&T) -> Box<dyn ControlGraphics> + 'static>,
    ) -> GraphicsRefWithControlFn<T> {
        GraphicsRefWithControlFn {
            label,
            graphics: self.clone(),
            control_graphic_fn,
        }
    }

    pub fn graphics(&self) -> GraphicsRef {
        self.clone()
    }

    pub fn control_graphics_fn<GraphicsConf>(
        &self,
    ) -> Option<GraphicsRefWithControlFn<GraphicsConf>> {
        None
    }

    pub fn cam(&self) -> Mat4 {
        let col = self.graphics.borrow().conf.input_vertex.view.view_proj;
        Mat4::from_cols_array_2d(&col)
    }

    pub fn update_tri(&mut self, c: &GraphicsWindowConf, tri: Triangulate) {
        // capture previous buffer sizes
        let (old_vert_bytes_len, old_index_bytes_len) = {
            let g = self.graphics.borrow();
            (
                bytemuck::cast_slice::<Vertex, u8>(&g.conf.input_vertex.vertices).len(),
                bytemuck::cast_slice::<u32, u8>(&g.conf.input_vertex.order).len(),
            )
        };

        {
            let mut g = self.graphics.borrow_mut();
            g.conf.input_vertex.vertices = tri
                .vertices
                .iter()
                .map(|x| Vertex::from_simple(x))
                .collect_vec();
            g.conf.input_vertex.order = tri.order.clone();
            let queue = c.device.queue();

            // vertex buffer: either recreate or overwrite
            let new_vert_bytes = bytemuck::cast_slice::<Vertex, u8>(&g.conf.input_vertex.vertices);
            if new_vert_bytes.len() > old_vert_bytes_len {
                // recreate vertex buffer with new size
                let vb = c
                    .device
                    .device()
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("vertex buffer"),
                        contents: new_vert_bytes,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
                g.vertex_buffers.vertex = vb;
            } else {
                queue.write_buffer(&g.vertex_buffers.vertex, 0, new_vert_bytes);
            }

            // index buffer with 4-byte alignment: recreate if growing
            const ALIGN: usize = 4;
            let raw_index = bytemuck::cast_slice::<u32, u8>(&g.conf.input_vertex.order);
            let (index_bytes, needs_recreate) = if raw_index.len() % ALIGN != 0 {
                // pad to alignment
                let pad = ALIGN - (raw_index.len() % ALIGN);
                let mut data = Vec::with_capacity(raw_index.len() + pad);
                data.extend_from_slice(raw_index);
                data.extend(std::iter::repeat(0).take(pad));
                (
                    data.into_boxed_slice(),
                    raw_index.len() + pad > old_index_bytes_len,
                )
            } else {
                (raw_index.into(), raw_index.len() > old_index_bytes_len)
            };
            if needs_recreate {
                let ib = c
                    .device
                    .device()
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("index buffer"),
                        contents: &index_bytes,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    });
                g.vertex_buffers.index = ib;
            } else {
                queue.write_buffer(&g.vertex_buffers.index, 0, &index_bytes);
            }
        }
    }
}

#[derive(Clone)]
pub struct GraphicsRefWithControlFn<GraphicsConf> {
    pub label: &'static str,
    pub graphics: GraphicsRef,
    pub control_graphic_fn: Arc<dyn Fn(&GraphicsConf) -> Box<dyn ControlGraphics>>,
}

impl<GraphicsConf> GraphicsRefWithControlFn<GraphicsConf> {
    pub fn control_graphics(&self, conf: &GraphicsConf) -> Vec<ControlGraphicsRef> {
        let ctrl_graphics = (self.control_graphic_fn)(conf);

        ControlGraphicsRef::new(self.label, ctrl_graphics, Some(self.graphics.clone()))
    }

    pub fn graphics(&self) -> GraphicsRef {
        self.graphics.clone()
    }

    pub fn control_graphics_fn(&self) -> Option<GraphicsRefWithControlFn<GraphicsConf>> {
        // Some(self.clone())
        let c = GraphicsRefWithControlFn {
            label: self.label,
            graphics: self.graphics.clone(),
            control_graphic_fn: self.control_graphic_fn.clone(),
        };
        Some(c)
    }
}

// for now this is just so nannou can create textures...
#[derive(Debug)]
pub struct TextureAndDesc {
    pub texture: Arc<wgpu::Texture>,
    pub desc: wgpu::TextureDescriptor<'static>,
}
impl TextureAndDesc {
    pub(crate) fn default_view(&self) -> wgpu::TextureView {
        self.texture.create_view(&Default::default())
    }
}

pub struct TextureFor3d {
    shadow_pipeline: wgpu::RenderPipeline,
    depth_view: wgpu::TextureView,
    shadow_view: wgpu::TextureView,
    shadow_sampler: wgpu::Sampler,
    shadow_bind_group: wgpu::BindGroup,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
}

// represents things needed to create a single texture... it's a bit of a mess
pub struct Graphics {
    name: String,
    conf: GraphicsCreator,
    bind_group: wgpu::BindGroup,
    vertex_buffers: VertexBuffers,
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
    textures_for_3d: Option<TextureFor3d>,
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
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::STORAGE_BINDING,
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
        is_3d: bool,
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

        if is_3d {
            // hrm, so hopefully won't have two inputs as well!

            // and finish up with vertex uniforms if we'll use them
            bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });

            bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            });

            bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            });
        }

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &bind_group_layout_entries,
        })
    }

    fn _sampler(device: &wgpu::Device, details: ShaderOptions) -> wgpu::Sampler {
        let sampler_desc = details.as_sampler_desc();
        let sampler = device.create_sampler(&sampler_desc);
        // println!("sampler: {:?}, {:?}", sampler, sampler_desc);
        sampler
    }

    fn _bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        input_texture_view: &wgpu::TextureView,
        input_texture_view_other: &Option<wgpu::TextureView>,
        initial_uniform_buffer: &wgpu::Buffer,
        initial_camera: Option<&wgpu::Buffer>,
        views_for_3d: &Option<TextureFor3d>,
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

        // if it's 3d, add the camera and light
        if let Some(cam) = initial_camera {
            assert!(binding_offset != 3); // shoulnd't have two texture for 3d!
            entries.push(wgpu::BindGroupEntry {
                binding: 3,
                resource: cam.as_entire_binding(),
            });

            // this should be set too, can make this nicer
            if let Some(v) = &views_for_3d {
                entries.push(wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&v.shadow_view),
                });

                entries.push(wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&v.shadow_sampler),
                });
            }
        }

        let bf: wgpu::BindGroupDescriptor = wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &entries,
        };

        device.create_bind_group(&bf)
    }

    fn _render_pipeline(
        conf: &GraphicsCreator,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        fs_mod: &wgpu::ShaderModule,
        dst_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let vertex_conf = &conf.input_vertex;
        let pipeline_layout = Graphics::_pipeline_layout(device, bind_group_layout);

        let vertex_buffer_layouts = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
        };

        let primitive = wgpu::PrimitiveState {
            topology: vertex_conf.topology,
            cull_mode: None,
            // cull_mode: Some(wgpu::Face::Back),
            ..wgpu::PrimitiveState::default()
        };

        let color_state = vec![Some(wgpu::ColorTargetState {
            format: dst_format,
            blend: Some(conf.blend_state()),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let depth_stencil = if vertex_conf.is_3d {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: Default::default(),
            })
        } else {
            None
        };

        let rp_desc = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_conf.vs_mod(device),
                entry_point: "main",
                buffers: &[vertex_buffer_layouts.clone()],
                #[cfg(not(feature = "nannou"))]
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive,
            depth_stencil,
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

        let main_pipeline = device.create_render_pipeline(&rp_desc);

        main_pipeline
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
        let conf_c = conf.clone();
        let has_second_texture = conf.second_texture.is_some();
        let details = conf.clone().details;
        let first_format = conf.first_texture.format;
        let second_format = conf.second_texture.map(|x| x.format);
        let dst_format = conf.dst_texture.format;

        let device = c.device.device();
        // todo, figure out msaa samples
        // let msaa_samples = 1;

        let fs_mod = shader_from_path(device, fs_shader_data);

        // make a bind group layout

        let first_texture_format = texture_src_path.to_format(first_format);
        let texture_and_desc = Graphics::texture(c.dims, device, first_texture_format);
        let input_texture = &texture_and_desc.texture;

        // maybe load the image source if we have one
        texture_src_path.maybe_load_texture(c.device, input_texture);

        let input_texture_view = input_texture.create_view(&Default::default());

        let (input_texture_view_other, other_texture_and_desc) = if has_second_texture {
            let other_texture = Graphics::texture(c.dims(), device, second_format.unwrap());
            (
                Some(other_texture.texture.create_view(&Default::default())),
                Some(other_texture),
            )
        } else {
            (None, None)
        };

        let sampler = Graphics::_sampler(device, details);
        let bind_group_layout = Graphics::_bind_group_layout(
            device,
            has_second_texture,
            false,
            conf.input_vertex.is_3d,
        );

        let initial_uniform_buffer = initial_uniform.to_buffer(device);

        let render_pipeline =
            Graphics::_render_pipeline(&conf, device, &bind_group_layout, &fs_mod, dst_format);

        let vertex_buffers = VertexBuffers::from_conf(device, &conf.input_vertex);

        let textures_for_3d = if conf.input_vertex.is_3d {
            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: c.dims[0],
                    height: c.dims[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                label: Some("Depth Texture"),
                view_formats: &[],
            });
            let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: c.dims[0],
                    height: c.dims[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("Shadow Texture"),
                view_formats: &[],
            });
            let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Shadow Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear, // make it smooth
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual), // for comparison!
                lod_min_clamp: 0.0,
                lod_max_clamp: 1.0,
                anisotropy_clamp: 1,
                border_color: Default::default(),
            });

            let shadow_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX, // | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("Shadow Bind Group Layout"),
                });

            let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &shadow_bind_group_layout, // Matches the shadow pipeline layout
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffers.uniform.as_entire_binding(),
                }],
                label: Some("Shadow Bind Group"),
            });

            // needs to be same
            let vertex_buffer_layouts = wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
            };

            let shadow_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Shadow Pipeline Layout"),
                    bind_group_layouts: &[&shadow_bind_group_layout], // This must match the bind groups used
                    push_constant_ranges: &[],
                });
            let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                layout: Some(&shadow_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &conf.input_vertex.shadow_vs_mod(device),
                    entry_point: "main",
                    buffers: &[vertex_buffer_layouts],
                    #[cfg(not(feature = "nannou"))]
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: None,
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less, // Closer depth wins
                    stencil: wgpu::StencilState::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                label: Some("shadow pipeline`"),
                multiview: None,
            });

            Some(TextureFor3d {
                depth_view,
                shadow_view,
                shadow_pipeline,
                shadow_sampler,
                shadow_bind_group,
                shadow_bind_group_layout,
            })
        } else {
            None
        };

        let bind_group = Graphics::_bind_group(
            device,
            &bind_group_layout,
            &input_texture_view,
            &input_texture_view_other,
            &initial_uniform_buffer,
            if conf.input_vertex.is_3d {
                Some(&vertex_buffers.uniform)
            } else {
                None
            },
            &textures_for_3d,
            &sampler,
        );

        Self {
            name,
            conf: conf_c,
            render_pipeline,
            bind_group,
            uniforms: initial_uniform,
            uniforms_buffer: initial_uniform_buffer,
            vertex_buffers,
            input_texture_view,
            input_texture_view_other,
            // things i might need to create custom bind groups later
            sampler,
            bind_group_layout,
            // things needed mostly for nannou right now..
            texture_and_desc,
            other_texture_and_desc,
            textures_for_3d,
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
            if self.conf.input_vertex.is_3d {
                Some(&self.vertex_buffers.uniform)
            } else {
                None
            },
            &self.textures_for_3d,
            &self.sampler,
        )
    }

    pub fn depth_stencil_attachment(&self) -> Option<wgpu::RenderPassDepthStencilAttachment> {
        if let Some(TextureFor3d { depth_view, .. }) = &self.textures_for_3d {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    #[cfg(feature = "nannou")]
                    store: true,
                    #[cfg(not(feature = "nannou"))]
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
        } else {
            None
        }
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
            // do the shadow pass if needed
            if let Some(TextureFor3d {
                shadow_view,
                shadow_pipeline,
                shadow_bind_group,
                ..
            }) = &self.textures_for_3d
            {
                let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Shadow Pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &shadow_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            #[cfg(not(feature = "nannou"))]
                            store: wgpu::StoreOp::Store,
                            #[cfg(feature = "nannou")]
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                    #[cfg(not(feature = "nannou"))]
                    occlusion_query_set: Default::default(),
                    #[cfg(not(feature = "nannou"))]
                    timestamp_writes: Default::default(),
                });
                shadow_pass.set_pipeline(shadow_pipeline);
                shadow_pass.set_bind_group(0, &shadow_bind_group, &[]);
                shadow_pass.set_vertex_buffer(0, self.vertex_buffers.vertex.slice(..));
                shadow_pass.set_index_buffer(
                    self.vertex_buffers.index.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                shadow_pass.draw_indexed(0..self.conf.input_vertex.indices(), 0, 0..1);
                drop(shadow_pass);
            }

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
                depth_stencil_attachment: self.depth_stencil_attachment(),
            };

            let mut rpass = encoder.begin_render_pass(&render_pass_desc);
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffers.vertex.slice(..));
            rpass.set_index_buffer(
                self.vertex_buffers.index.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            rpass.draw_indexed(0..self.conf.input_vertex.indices(), 0, 0..1);
            drop(rpass);
        }

        device_state.queue().submit(Some(encoder.finish()));
    }

    pub fn render(&self, device: &DeviceState, output_texture_view: &wgpu::TextureView) {
        self.render_with_custom_bind_group(device, output_texture_view, &self.bind_group)
    }

    pub fn update_view(&self, c: &GraphicsWindowConf, view: Mat4, light: Mat4) {
        self.vertex_buffers.update_view(c, view, light);
    }
}

pub fn quick_texture(dims: [u32; 2], device: &wgpu::Device) -> TextureAndDesc {
    Graphics::texture(dims, device, DEFAULT_TEXTURE_FORMAT)
}

pub struct VertexBuffers {
    vertex: wgpu::Buffer,  // sets the vertices of the thing
    index: wgpu::Buffer,   // used with vertices to form triangles, using TriangleList primative
    uniform: wgpu::Buffer, // this we update to change the camera!
}

impl VertexBuffers {
    // inits them all
    fn from_conf(device: &wgpu::Device, conf: &InputVertexConf) -> Self {
        let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&conf.vertices[..]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let order = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&conf.order[..]),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        let uniform = conf.view.to_buffer(device);

        Self {
            vertex,
            index: order,
            uniform,
        }
    }
    fn update_view(&self, c: &GraphicsWindowConf, view: Mat4, light: Mat4) {
        let queue = c.device.queue();
        // self.conf.set_view(m); // hmm, running into borrow things here
        let v = VertexUniforms::from_mat4(view, light);

        // queue.write_buffer(&self.uniform, 0, v.as_bytes());
        v.write_buffer(&self.uniform, queue);
    }
}
