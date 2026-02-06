#![allow(dead_code)]
use std::{cell::RefCell, collections::HashMap, fs, path::PathBuf, rc::Rc};

use glam::Vec2;
use murrelet_common::{triangulate::DefaultVertex, MurreletTime};
use serde::Serialize;

use crate::{
    compute::ComputeGraphicsToTextureRef,
    device_state::{DeviceStateForRender, GraphicsAssets},
    gpu_livecode::{AnyControlRef, ControlProvider},
    graphics_ref::{
        AnyGraphicsRef, Graphics, GraphicsCreator, GraphicsRefCustom, GraphicsVertex,
        DEFAULT_LOADED_TEXTURE_FORMAT,
    },
    shader_str::*,
    window::GraphicsWindowConf,
};

#[cfg(feature = "nannou")]
use wgpu_for_nannou as wgpu;

#[cfg(not(feature = "nannou"))]
use wgpu_for_latest as wgpu;

// const DEFAULT_LOADED_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Uint;

const DEFAULT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// For example
/// Examples:
///
///    build_shader!{
///     raw r###"
///     fn shape(in: vec3<f32>) -> f32 {
///         let rep_period = vec3<f32>(10.0);
///         let q: vec3<f32> = (mod(in + 0.5 * rep_period, rep_period)) - 0.5 * rep_period;
///         let sphere = length(q) - 1.5;

///         // let sphere = length(in) - 3.2;
///         return sphere;
///     }
///     "###;
///     use "ray_march"
///    }
///
///

#[macro_export]
macro_rules! build_shader_custom_vertex {
    (@parse ()) => {{""}}; // done!

    // for raw things in the prefix
    (@parse (raw $raw:expr;$($tail:tt)*)) => {
        {
            let rest = build_shader!(@parse ($($tail)*));
            format!("{}\n{}", $raw, rest)
        }
    };

    (@parsecode ()) => {{""}}; // done!

    (@parsecode (raw $raw:expr;$($tail:tt)*)) => {
        {
            let rest = build_shader!(@parsecode ($($tail)*));
            format!("{}\n{}", $raw, rest)
        }
    };

    // wrap the main code itself in ()
    (@parse ((prefix $prefix:expr; $($tail:tt)*))) => {
        {
            //let prefix = ShaderStr::Prefix.to_str();
            let rest = build_shader!(@parsecode ($($tail)*));
            let suffix = ShaderStr::Suffix.to_str();
            format!("{}\n{}\n{}", $prefix, rest, suffix)
        }
    }; // includes

    // arm for funky parsing
    (@parse $($raw:tt)*) => {
        {
            println!("???");
            "???"
            // unreachable!();
        }
    };

    // capture the initial one
    ($($raw:tt)*) => {
        {
            format!(
                "{}\n{}\n{}",
                ShaderStr::Binding1Tex.to_str(),
                ShaderStr::Includes.to_str(),
                build_shader_custom_vertex!(@parse ($($raw)*)),
            )
        }
    };
}

#[macro_export]
macro_rules! build_shader {
    (@parse ()) => {{""}}; // done!

    // includes, should figure out how to include strs now..
    // (@parse (use $loc:expr;$($tail:tt)*)) => {
    //     {
    //         let data = load_macro_shader!($loc);
    //         let rest = build_shader!(@parse ($($tail)*));
    //         format!("{}\n{}", data, rest)
    //     }
    // };

    // for raw things in the prefix
    (@parse (raw $raw:expr;$($tail:tt)*)) => {
        {
            let rest = build_shader!(@parse ($($tail)*));
            format!("{}\n{}", $raw, rest)
        }
    };

    (@parsecode ()) => {{""}}; // done!

    (@parsecode (raw $raw:expr;$($tail:tt)*)) => {
        {
            let rest = build_shader!(@parsecode ($($tail)*));
            format!("{}\n{}", $raw, rest)
        }
    };

    // wrap the main code itself in ()
    (@parse (($($tail:tt)*))) => {
        {
            let prefix = ShaderStr::Prefix.to_str();
            let rest = build_shader!(@parsecode ($($tail)*));
            let suffix = ShaderStr::Suffix.to_str();
            format!("{}\n{}\n{}", prefix, rest, suffix)
        }
    }; // includes

    // arm for funky parsing
    (@parse $($raw:tt)*) => {
        {
            println!("???");
            "???"
            // unreachable!();
        }
    };

    // capture the initial one
    ($($raw:tt)*) => {
        {
            format!(
                "{}\n{}\n{}",
                ShaderStr::Binding1Tex.to_str(),
                ShaderStr::Includes.to_str(),
                build_shader!(@parse ($($raw)*)),
            )
        }
    };
}

// i think i originally added this so i could use it?
pub enum ShaderStr {
    Binding1Tex,
    Binding2Tex,
    Binding3d,
    Compute,
    Includes,
    Prefix,
    Suffix,
    ComputeFormatStr,
}
impl ShaderStr {
    pub fn to_str(&self) -> &str {
        match self {
            ShaderStr::Binding1Tex => BINDING_1TEX,
            ShaderStr::Binding2Tex => BINDING_2TEX,
            ShaderStr::Compute => COMPUTE_TEX,
            ShaderStr::Includes => INCLUDES,
            ShaderStr::Prefix => PREFIX,
            ShaderStr::Suffix => SUFFIX,
            ShaderStr::Binding3d => BINDING_3D,
            ShaderStr::ComputeFormatStr => COMPUTE_FORMAT_STR,
        }
    }
}

#[macro_export]
macro_rules! build_shader_2tex {
    // capture the initial one
    ($($raw:tt)*) => {
        {
            format!(
                "{}\n{}\n{}",
                ShaderStr::Binding2Tex.to_str(),
                ShaderStr::Includes.to_str(),
                build_shader!(@parse ($($raw)*)),
            )
        }
    };
}

#[macro_export]
macro_rules! build_shader_3d {
    ($($raw:tt)*) => {
        {
            format!(
                "{}\n{}\n{}",
                ShaderStr::Binding3d.to_str(),
                ShaderStr::Includes.to_str(),
                build_shader!(@parse ($($raw)*)),
            )
        }
    };
}

// input_structure can be
// struct Input {
//   a: vec2<f32>;
//   b: vec2<f32>;
// }

//COMPUTE_FORMAT_STR.replace("#PREFIX_CODEHERE", prefix).replace("#FORLOOP_CODEHERE#", forloop).replace("#SUFFIX_CODEHERE#", suffix);

#[macro_export]
macro_rules! build_compute_shader {
    // capture the initial one
    ($input_structure:tt, $prefix:tt, $forloop:tt, $suffix:tt) => {{
        format!(
            "{}\n{}\n{}\n{}",
            $input_structure,
            ShaderStr::Compute.to_str(),
            ShaderStr::Includes.to_str(),
            ShaderStr::ComputeFormatStr
                .to_str()
                .replace("#PREFIX_CODEHERE#", $prefix)
                .replace("#FORLOOP_CODEHERE#", $forloop)
                .replace("#SUFFIX_CODEHERE#", $suffix)
        )
    }};
}

#[derive(Serialize)]
pub struct RenderDebugPrint {
    pub src: String,
    pub dest: String,
}

#[derive(Serialize)]
pub struct WrappedRenderDebugPrint {
    pub idx: String,
    pub r: RenderDebugPrint,
}

pub trait RenderTrait {
    fn render(&self, device_state_for_render: &DeviceStateForRender);
    fn debug_print(&self) -> Vec<RenderDebugPrint>;

    fn dest_view(&self) -> Option<wgpu::TextureView>;
    fn dest_view_other(&self) -> Option<wgpu::TextureView>;

    // hmm, i don't know how to do this with the boxes
    fn is_choice(&self) -> bool {
        false
    }

    fn adjust_choice(&mut self, _choice_val: usize) {}
}

pub struct SimpleRender<VertexKindSource, VertexKindDest> {
    pub source: GraphicsRefCustom<VertexKindSource>,
    pub dest: GraphicsRefCustom<VertexKindDest>,
}
impl<VertexKindSource: GraphicsVertex, VertexKindDest: GraphicsVertex>
    SimpleRender<VertexKindSource, VertexKindDest>
{
    pub fn new_box(
        source: GraphicsRefCustom<VertexKindSource>,
        dest: GraphicsRefCustom<VertexKindDest>,
    ) -> Box<SimpleRender<VertexKindSource, VertexKindDest>> {
        Box::new(SimpleRender { source, dest })
    }

    fn dest(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }
}

impl<VertexKindSource, VertexKindDest> RenderTrait
    for SimpleRender<VertexKindSource, VertexKindDest>
where
    VertexKindSource: GraphicsVertex,
    VertexKindDest: GraphicsVertex,
{
    fn render(&self, device: &DeviceStateForRender) {
        self.source
            .render(device.device_state(), &self.dest.texture_view());
    }

    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        vec![RenderDebugPrint {
            src: self.source.name(),
            dest: self.dest.name(),
        }]
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        self.dest.texture_view_other()
    }

    fn is_choice(&self) -> bool {
        false
    }

    fn adjust_choice(&mut self, _choice_val: usize) {}
}

pub struct TwoSourcesRender<VertexKind: GraphicsVertex> {
    pub source_main: GraphicsRefCustom<VertexKind>,
    pub source_other: GraphicsRefCustom<VertexKind>,
    pub dest: GraphicsRefCustom<VertexKind>,
}
impl<VertexKind: GraphicsVertex> TwoSourcesRender<VertexKind> {
    pub fn new_box(
        source_main: GraphicsRefCustom<VertexKind>,
        source_other: GraphicsRefCustom<VertexKind>,
        dest: GraphicsRefCustom<VertexKind>,
    ) -> Box<TwoSourcesRender<VertexKind>> {
        Box::new(TwoSourcesRender {
            source_main,
            source_other,
            dest,
        })
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        self.dest.texture_view_other()
    }
}

impl<VertexKind: GraphicsVertex> RenderTrait for TwoSourcesRender<VertexKind> {
    fn render(&self, device: &DeviceStateForRender) {
        self.source_main
            .render(device.device_state(), &self.dest_view().unwrap());
        self.source_other
            .render(device.device_state(), &self.dest_view_other().unwrap());
    }

    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        vec![
            RenderDebugPrint {
                src: self.source_main.name(),
                dest: self.dest.name(),
            },
            RenderDebugPrint {
                src: self.source_other.name(),
                dest: self.dest.name(),
            },
        ]
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        self.dest.texture_view_other()
    }
}

// holds a gpu pipeline :O
pub struct PipelineRender<
    GraphicsConf,
    VertexKindSource: GraphicsVertex,
    VertexKindDest: GraphicsVertex,
> {
    pub source: GraphicsRefCustom<VertexKindSource>,
    pub pipeline: GPUPipelineRef<GraphicsConf>,
    pub dest: GraphicsRefCustom<VertexKindDest>,
}

impl<GraphicsConf, VertexKindSrc: GraphicsVertex, VertexKindDest: GraphicsVertex>
    PipelineRender<GraphicsConf, VertexKindSrc, VertexKindDest>
{
    pub fn new_box(
        source: GraphicsRefCustom<VertexKindSrc>,
        pipeline: GPUPipelineRef<GraphicsConf>,
        dest: GraphicsRefCustom<VertexKindDest>,
    ) -> Box<Self> {
        Box::new(Self {
            source,
            pipeline,
            dest,
        })
    }
}
impl<GraphicsConf, VertexKindSrc: GraphicsVertex, VertexKindDest: GraphicsVertex> RenderTrait
    for PipelineRender<GraphicsConf, VertexKindSrc, VertexKindDest>
{
    fn render(&self, device_state_for_render: &DeviceStateForRender) {
        // write source to pipeline source
        self.source.render(
            device_state_for_render.device_state(),
            &self.pipeline.source(),
        );
        self.pipeline.render(device_state_for_render);
    }

    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        self.pipeline.debug_print()
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        self.dest.texture_view_other()
    }
}

// given a list of inputs, choose which one to use
pub struct ChoiceRender<VertexKind: GraphicsVertex> {
    pub sources: Vec<GraphicsRefCustom<VertexKind>>,
    pub dest: GraphicsRefCustom<VertexKind>,
    choice: usize,
}
impl<VertexKind: GraphicsVertex> ChoiceRender<VertexKind> {
    pub fn new_box(
        sources: Vec<GraphicsRefCustom<VertexKind>>,
        dest: GraphicsRefCustom<VertexKind>,
    ) -> Box<ChoiceRender<VertexKind>> {
        Box::new(ChoiceRender {
            sources,
            dest,
            choice: 0,
        })
    }
}

impl<VertexKind: GraphicsVertex> RenderTrait for ChoiceRender<VertexKind> {
    fn render(&self, device: &DeviceStateForRender) {
        let source = &self.sources[self.choice % self.sources.len()];
        let dest = &self.dest;
        source.render(device.device_state(), &dest.texture_view());
    }

    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        // let source_names = self.sources.borrow_mut();
        // let dest = self.dest.borrow_mut();
        // vec![RenderDebugPrint{src: source_main.name.clone(), dest: dest.name.clone()}, RenderDebugPrint{src: source_other.name.clone(), dest: dest.name.clone()}]
        todo!()
    }

    fn is_choice(&self) -> bool {
        true
    }

    // wraps if wrong
    fn adjust_choice(&mut self, choice_val: usize) {
        self.choice = choice_val % self.sources.len()
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        None
    }
}

pub struct PingPongRender<VertexKind: GraphicsVertex> {
    pub k: usize,
    pub ping: GraphicsRefCustom<VertexKind>, // it'll end up here
    pub pong: GraphicsRefCustom<VertexKind>,
}

impl<VertexKind: GraphicsVertex> PingPongRender<VertexKind> {
    pub fn new_box(
        k: usize,
        ping: GraphicsRefCustom<VertexKind>,
        pong: GraphicsRefCustom<VertexKind>,
    ) -> Box<PingPongRender<VertexKind>> {
        Box::new(PingPongRender { k, ping, pong })
    }
}

impl<VertexKind: GraphicsVertex> RenderTrait for PingPongRender<VertexKind> {
    fn render(&self, device: &DeviceStateForRender) {
        let ping_texture = &self.ping.texture_view();
        let pong_texture = &self.pong.texture_view();
        for _ in 0..self.k {
            self.ping
                .graphics()
                .render(device.device_state(), pong_texture);
            self.pong
                .graphics()
                .render(device.device_state(), ping_texture);
        }
    }

    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        let ping = &self.ping;
        let pong = &self.pong;
        vec![
            RenderDebugPrint {
                src: ping.name(),
                dest: pong.name(),
            },
            RenderDebugPrint {
                src: pong.name(),
                dest: ping.name(),
            },
            RenderDebugPrint {
                src: ping.name(),
                dest: pong.name(),
            },
            RenderDebugPrint {
                src: pong.name(),
                dest: ping.name(),
            },
        ]
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.pong.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        None
    }
}
pub struct ComputeTextureRender<VertexKind: GraphicsVertex> {
    pub source: ComputeGraphicsToTextureRef,
    pub dest: GraphicsRefCustom<VertexKind>,
}

impl<VertexKind: GraphicsVertex> ComputeTextureRender<VertexKind> {
    pub fn new_box(
        source: ComputeGraphicsToTextureRef,
        dest: GraphicsRefCustom<VertexKind>,
    ) -> Box<Self> {
        Box::new(Self { source, dest })
    }
}

impl<VertexKind: GraphicsVertex> RenderTrait for ComputeTextureRender<VertexKind> {
    // whenver it's called, it'll increment! check if it's overdue before rendering!
    fn render(&self, device_state_for_render: &DeviceStateForRender) {
        let source_texture = &self.source;
        let dest = &self.dest;
        source_texture.render(device_state_for_render, dest);
    }

    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        let source = &self.source;
        let dest = &self.dest;
        vec![RenderDebugPrint {
            src: source.name(),
            dest: dest.name(),
        }]
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        // todo!()
        None
    }
}

pub struct DisplayRender<VertexKind> {
    pub source: GraphicsRefCustom<VertexKind>,
}

impl<VertexKind: GraphicsVertex> DisplayRender<VertexKind> {
    pub fn new_box(source: GraphicsRefCustom<VertexKind>) -> Box<DisplayRender<VertexKind>> {
        Box::new(DisplayRender { source })
    }
}

impl<VertexKind: GraphicsVertex> RenderTrait for DisplayRender<VertexKind> {
    fn render(&self, device: &DeviceStateForRender) {
        let source = &self.source;
        source.render_to_texture(device.device_state(), device.display_view());
    }
    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        let source = &self.source;
        vec![RenderDebugPrint {
            src: source.name(),
            dest: "output!".to_string(),
        }]
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        None
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        None
    }
}

pub struct GPUPipeline<GraphicConf> {
    pub dag: Vec<Box<dyn RenderTrait>>,
    choices: Vec<usize>,
    names: HashMap<String, Box<dyn AnyGraphicsRef>>, // todo, do i need this with ctrl?
    ctrl: Vec<Box<dyn ControlProvider<GraphicConf>>>,
    source: Option<String>,
}

impl<GraphicConf> GPUPipeline<GraphicConf> {
    pub fn new() -> GPUPipeline<GraphicConf> {
        GPUPipeline {
            dag: Vec::new(),
            choices: Vec::new(),
            names: HashMap::new(),
            ctrl: Vec::new(),
            source: None,
        }
    }

    pub fn add_control_graphics<P>(&mut self, _label: &str, provider: P)
    where
        P: ControlProvider<GraphicConf> + 'static,
    {
        self.ctrl.push(Box::new(provider));
    }

    pub fn control_graphics(&self, t: &GraphicConf) -> Vec<Box<dyn AnyControlRef>> {
        let mut out = Vec::new();
        for p in &self.ctrl {
            out.extend(p.make_controls(t));
        }
        out
    }

    pub fn set_source(&mut self, src: &str) {
        self.source = Some(src.to_string());
    }

    pub fn add_step(&mut self, d: Box<dyn RenderTrait>) {
        let curr_idx = self.dag.len();

        {
            // handle the special case of choices, where we should register it
            if d.is_choice() {
                self.choices.push(curr_idx);
            }
        }

        self.dag.push(d);
    }

    pub fn add_label<VertexKind>(&mut self, name: &str, g: GraphicsRefCustom<VertexKind>)
    where
        VertexKind: GraphicsVertex + 'static,
    {
        self.names.insert(name.to_string(), Box::new(g));
    }

    pub fn get_graphic(&self, name: &str) -> Option<&dyn AnyGraphicsRef> {
        self.names.get(name).map(|g| g.as_ref())
    }

    // no-op if it doesn't exist
    pub fn adjust_choice(&mut self, choice_idx: usize, choice_val: usize) {
        // use the choice idx to find the right one.
        if choice_idx < self.choices.len() {
            self.dag[self.choices[choice_idx]].adjust_choice(choice_val);
        } else {
            println!("what, that choice {:?} doesn't exist", choice_idx);
        }
    }

    pub fn render(&self, device: &DeviceStateForRender) {
        self.dag.iter().for_each(|x| x.render(device))
    }

    pub fn debug_print(&self) -> Vec<RenderDebugPrint> {
        self.dag.iter().flat_map(|x| x.debug_print()).collect()
    }

    fn source(&self) -> wgpu::TextureView {
        // hm this should happen on start
        let name = self
            .source
            .as_ref()
            .expect("should have set a source if you're gonna get it source");
        self.get_graphic(name)
            .unwrap_or_else(|| panic!("gave a source {} that doesn't exist", name))
            .texture_view()
    }
}

impl<GraphicsConf> Default for GPUPipeline<GraphicsConf> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct GPUPipelineRef<GraphicsConf>(Rc<RefCell<GPUPipeline<GraphicsConf>>>);

impl<GraphicsConf> GPUPipelineRef<GraphicsConf> {
    pub fn new(pipeline: GPUPipeline<GraphicsConf>) -> Self {
        GPUPipelineRef(Rc::new(RefCell::new(pipeline)))
    }

    pub fn render(&self, device: &DeviceStateForRender) {
        self.0.borrow().render(device)
    }

    pub fn debug_print(&self) -> Vec<RenderDebugPrint> {
        self.0.borrow().debug_print()
    }

    pub fn source(&self) -> wgpu::TextureView {
        self.0.borrow().source()
    }

    pub fn control_graphics(&self, conf: &GraphicsConf) -> Vec<Box<dyn AnyControlRef>> {
        self.0.borrow().control_graphics(conf)
    }
}

pub struct SingleTextureRender {
    pub source: ImageTextureRef,
    pub dest: GraphicsRefCustom<DefaultVertex>,
}

impl SingleTextureRender {
    pub fn new_box(
        source: ImageTextureRef,
        dest: GraphicsRefCustom<DefaultVertex>,
    ) -> Box<SingleTextureRender> {
        Box::new(SingleTextureRender { source, dest })
    }
}

impl RenderTrait for SingleTextureRender {
    // whenver it's called, it'll increment! check if it's overdue before rendering!
    fn render(&self, device_state_for_render: &DeviceStateForRender) {
        let source_texture = &self.source;
        let dest = &self.dest;
        source_texture.render(device_state_for_render, &dest.texture_view());
    }

    fn debug_print(&self) -> Vec<RenderDebugPrint> {
        let source = &self.source;
        let dest = &self.dest;
        vec![RenderDebugPrint {
            src: source.name(),
            dest: dest.name(),
        }]
    }

    fn dest_view(&self) -> Option<wgpu::TextureView> {
        Some(self.dest.texture_view())
    }

    fn dest_view_other(&self) -> Option<wgpu::TextureView> {
        None
    }
}

// makes it easier to ad control grpahics
#[macro_export]
macro_rules! with_control_graphics {
    ($name:ident = $instance:expr, |$param:ident: $ttype:ident| $body:expr) => {
        let $name = $instance.with_control_graphics(
            stringify!($name),
            Arc::new(|$param: &$ttype| Box::new($body) as Box<dyn ControlGraphics>),
        );
    };
}

// this is basically ran for every node, so can add label and such
#[macro_export]
macro_rules! pipeline_add_label {
    ($pipeline:ident, $val:ident) => {{
        $pipeline.add_label(stringify!($val), $val.graphics());
        if let Some(ctrl) = $val.control_graphics_fn() {
            $pipeline.add_control_graphics(stringify!($val), ctrl);
        }
    }};
}

#[macro_export]
macro_rules! build_shader_pipeline {
    () => {}; // empty
    (@parse $pipeline:ident ()) => {}; // done!

    // write to display: a -> DISPLAY, this is the view that will be passed in the pipeline render call
    (@parse $pipeline:ident ($source:ident -> DISPLAY;$($tail:tt)*)) => {
        {
            println!("add display");
            $pipeline.add_step(
                DisplayRender::new_box(
                    $source.graphics(),
                )
            );
            pipeline_add_label!($pipeline, $source);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // process texture render: *a -> t
    (@parse $pipeline:ident (*$source:ident -> $dest:ident;$($tail:tt)*)) => {
        {
            println!("add display");
            $pipeline.add_step(
                TextureRender::new_box(
                    $source.graphics(),
                    $dest.graphics(),
                )
            );
            pipeline_add_label!($pipeline, $dest);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // process single texture: +a -> t
    (@parse $pipeline:ident (+$source:ident -> $dest:ident;$($tail:tt)*)) => {
        {
            println!("add display");
            $pipeline.add_step(
                SingleTextureRender::new_box(
                    $source.clone(),
                    $dest.graphics(),
                )
            );
            pipeline_add_label!($pipeline, $dest);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // process ping pong render: (a <-> b) -> t
    (@parse $pipeline:ident (($ping:ident <-> $pong:ident) * $count:expr;$($tail:tt)*)) => {
        {
            println!("add ping pong");
            $pipeline.add_step(
                PingPongRender::new_box(
                    $count,
                    $ping.graphics(),
                    $pong.graphics())
            );
            pipeline_add_label!($pipeline, $ping);
            pipeline_add_label!($pipeline, $pong);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // process choice render: [a | b | c] -> t
    (@parse $pipeline:ident ([$source:ident$( | $source_rest:ident)*] -> $dest:ident;$($tail:tt)*)) => {
        {
            println!("add choice render $dest");
            $pipeline.add_step(
                ChoiceRender::new_box(
                    // todo, allow for more than two
                    vec![
                        $source.graphics(),
                        $($source_rest.graphics(), )*
                    ],
                    $dest.graphics()
                )
            );
            pipeline_add_label!($pipeline, $source);
            $(pipeline_add_label!($pipeline, $source_rest);)*
            pipeline_add_label!($pipeline, $dest);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // two sources to output: (a, b) -> t
    (@parse $pipeline:ident (($source1:ident, $source2:ident) -> $dest:ident;$($tail:tt)*)) => {
        {
            println!("add two sources");

            $pipeline.add_step(
                TwoSourcesRender::new_box(
                    $source1.graphics(),
                    $source2.graphics(),
                    $dest.graphics())
            );
            pipeline_add_label!($pipeline, $source1);
            pipeline_add_label!($pipeline, $source2);
            pipeline_add_label!($pipeline, $dest);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // one source to create one graphicsref: a -> T => t;
    (@parse $pipeline:ident ($source:ident -> $subpipe:ident => $dest:ident;$($tail:tt)*)) => {
        {
            println!("add pipeline");
            let $dest = $subpipe.out().clone();
            $pipeline.add_step(
                PipelineRender::new_box(
                    $source.graphics(),
                    $subpipe.gpu_pipeline(),
                    $dest.graphics()
                )
            );
            pipeline_add_label!($pipeline, $source);
            pipeline_add_label!($pipeline, $dest);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // compute shaders: =a -> T
    (@parse $pipeline:ident (=$source:ident -> $dest:ident;$($tail:tt)*)) => {
        {
            println!("add compute");
            $pipeline.add_step(
                ComputeTextureRender::new_box(
                    $source.clone(),
                    $dest.graphics()
                )
            );
            // pipeline_add_label!($pipeline, $source);
            pipeline_add_label!($pipeline, $dest);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // one source to output: a -> t
    (@parse $pipeline:ident ($source:ident -> $dest:ident;$($tail:tt)*)) => {
        {
            println!("add simple");
            $pipeline.add_step(
                SimpleRender::new_box(
                    $source.graphics(),
                    $dest.graphics()
                )
            );
            pipeline_add_label!($pipeline, $source);
            pipeline_add_label!($pipeline, $dest);

            build_shader_pipeline!(@parse $pipeline ($($tail)*));
        }
    };

    // arm for funky parsing
    (@parse $($raw:tt)*) => {
        {
            println!("???");
            unreachable!();
        }
    };

    // capture the initial one and prefix it with @parse
    ($($raw:tt)*) => {
        {
            println!("new pipeline!");
            let mut pipeline = GPUPipeline::new();  // create our new pipeline
            build_shader_pipeline!(@parse pipeline ($($raw)*));
            pipeline
        }
    };
}

#[derive(Clone)]
pub struct ImageTextureRef(Rc<RefCell<ImageTexture>>);
impl ImageTextureRef {
    pub fn render(&self, device_state_for_render: &DeviceStateForRender, dest: &wgpu::TextureView) {
        self.0.borrow().render(device_state_for_render, dest)
    }
    pub fn name(&self) -> String {
        self.0.borrow().graphics.name()
    }

    pub fn from_image_texture(im: ImageTexture) -> Self {
        Self(Rc::new(RefCell::new(im)))
    }
}

#[derive(Clone)]
pub struct VideoTextureRef(Rc<RefCell<VideoTexture>>);
pub struct VideoTexture {
    name: String,
    pub graphics: GraphicsRefCustom<DefaultVertex>,
    pub binds: Vec<wgpu::BindGroup>, // path to pngs, probably keep it smapp
    pub fps: u64,
    last_time: Option<MurreletTime>,
    curr_i: usize, // current index in src_paths, starts at 0
}

impl VideoTexture {
    // todo, how to make this work on web?
    fn _load_path(path: &PathBuf, boomerang: bool) -> Vec<PathBuf> {
        println!("reading path {:?}", path);
        let paths = fs::read_dir(path).unwrap();
        let mut p = paths
            .filter_map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                let metadata = fs::metadata(&path).unwrap();

                if metadata.is_file()
                    && path
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|y| y == "png")
                        .unwrap_or(false)
                {
                    println!("{:?}", entry);
                    Some(path)
                } else {
                    None
                }
            })
            .collect::<Vec<PathBuf>>();
        p.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        if boomerang {
            // play back and forth, but don't repeat frames
            let mut p2 = p.clone();
            p2.reverse();
            p2.pop();
            p2.remove(0);

            p.append(&mut p2);
        }
        p
    }

    pub fn overdue_for_an_update(&self) -> bool {
        self.last_time
            // .map(|x| right_now() - x >= (1000 / self.fps as u128))
            .map(|x| (MurreletTime::now() - x).as_millis_u128() >= (1000 / self.fps as u128))
            .unwrap_or(true)
    }

    pub fn next_frame(&mut self) {
        self.curr_i = (self.curr_i + 1) % self.binds.len();
        self.last_time = Some(MurreletTime::now());
    }

    pub fn render(
        &self,
        device_state_for_render: &DeviceStateForRender,
        output_texture_view: &wgpu::TextureView,
    ) {
        let bind_group = &self.binds[self.curr_i];
        self.graphics.render_with_custom_bind_group(
            device_state_for_render.device_state(),
            output_texture_view,
            bind_group,
        )
    }

    pub fn new_mut(
        c: &GraphicsWindowConf,
        name: &str,
        path: &[&str],
        fps: u64,
        boomerang: bool,
    ) -> VideoTextureRef {
        VideoTextureRef(Rc::new(RefCell::new(VideoTexture::new(
            c, name, path, fps, boomerang,
        ))))
    }

    pub fn new(
        c: &GraphicsWindowConf,
        name: &str,
        raw_path: &[&str],
        fps: u64,
        boomerang: bool,
    ) -> VideoTexture {
        let device = c.device();

        let assets_path = c.assets_path.force_path_buf();

        let mut path = assets_path;
        for loc in raw_path {
            path = path.join(loc);
        }

        let src_paths = VideoTexture::_load_path(&path, boomerang);
        assert!(src_paths.len() < 61); // i don't know, i'm just scared

        // load one as dummy to get image
        // let source_dims = wgpu::Texture::from_path(c.window, &src_paths[0]).unwrap().size();
        let source_dims = c.dims; // ??

        // let target_dims =  _dims_from_window(c);
        let target_dims = c.dims;

        let gradient_shader: String = build_shader! {
            (
                raw r###"
                let multiplier = uniforms.more_info.x;
                let source = uniforms.more_info_other.xy;
                let targ = uniforms.more_info_other.za;

                let multi = targ / source / multiplier;

                let result: vec4<f32> = textureSample(tex, tex_sampler, tex_coords * multi);
                "###;
            )
        };

        // let _uniforms = BasicUniform::from_dims(c.dims);

        let conf = GraphicsCreator::default()
            .with_first_texture_format(DEFAULT_TEXTURE_FORMAT)
            .with_dst_format(DEFAULT_TEXTURE_FORMAT)
            .with_mag_filter(wgpu::FilterMode::Linear)
            .with_address_mode(wgpu::AddressMode::Repeat);

        let graphics = GraphicsRefCustom::new(name, c, &gradient_shader, &conf);
        graphics.update_uniforms_other(
            c,
            [1.0, 0.0, 0.0, 0.0],
            [
                source_dims[0] as f32,
                source_dims[1] as f32,
                target_dims[0] as f32,
                target_dims[1] as f32,
            ],
        );

        // todo, move this into GraphicsAssets
        let binds = src_paths
            .iter()
            .map(|path| {
                // let texture = wgpu::Texture::from_path(c.window, path).unwrap(); // load the path
                let texture_and_desc = Graphics::<DefaultVertex>::texture(
                    source_dims,
                    c.device(),
                    DEFAULT_LOADED_TEXTURE_FORMAT,
                );
                GraphicsAssets::LocalFilesystem(path.to_path_buf())
                    .maybe_load_texture(c.device, &texture_and_desc.texture);
                let texture_view =
                    texture_and_desc
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor {
                            ..Default::default()
                        });
                println!("texture {:?}", texture_view);
                graphics
                    .graphics
                    .borrow()
                    .make_new_custom_bind_group(device, &texture_view)
            })
            .collect();

        Self {
            name: name.to_string(),
            graphics,
            binds,
            fps,
            last_time: None,
            curr_i: 0,
        }
    }
}

pub struct ImageTexture {
    name: String,
    pub graphics: GraphicsRefCustom<DefaultVertex>,
}

impl ImageTexture {
    pub fn render(
        &self,
        device_state_for_render: &DeviceStateForRender,
        other: &wgpu::TextureView,
    ) {
        self.graphics
            .render(device_state_for_render.device_state(), other);
    }

    pub fn new_mut(
        name: &str,
        path: &PathBuf,
        c: &GraphicsWindowConf,
        address_mode: wgpu::AddressMode,
    ) -> ImageTextureRef {
        ImageTexture::new_mut_with_dims(name, path, c, address_mode)
    }

    pub fn new_mut_with_dims(
        name: &str,
        path: &PathBuf,
        c: &GraphicsWindowConf,
        address_mode: wgpu::AddressMode,
    ) -> ImageTextureRef {
        ImageTextureRef(Rc::new(RefCell::new(ImageTexture::new(
            name,
            path,
            c,
            address_mode,
        ))))
    }

    pub fn new(
        name: &str,
        src_path: &PathBuf,
        c: &GraphicsWindowConf,
        address_mode: wgpu::AddressMode,
    ) -> Self {
        // load one as dummy to get image
        // let source_dims = wgpu::Texture::from_path(c.window, src_path).unwrap().size();

        // hrm, when this was set to width/height, it didn't work, it shrunk the whole thing..
        // let (_, width, height) = crate::device_state::check_img_size(src_path).unwrap();
        let source_dims = c.dims; //[width, height]; // c.dims; // ??
        let target_dims = c.dims;
        println!("source: {:?} {:?}", source_dims, target_dims);

        let repeat_img: String = build_shader! {
            (
                raw r###"
                // the sizes of the input and output maps are in pixels
                let entire_source_size_pxl = uniforms.more_info_other.xy;
                let target_size_pxl = uniforms.more_info_other.zw;

                // let aspect = vec2<f32>(uniforms.dims.x / uniforms.dims.y);

                let source_normalized_dims = vec2<f32>(1.0 / entire_source_size_pxl.x, 1.0 / entire_source_size_pxl.y); //uniforms.dims.zw;

                // grab the intended size of the source window and offset.
                let windowed_source_size_pxl = uniforms.more_info.zw;
                let windowed_source_offset_pxl = uniforms.more_info.xy;

                let windowed_source_offset_txl = windowed_source_offset_pxl * source_normalized_dims;

                // how much of the source image should we sample?
                let window_to_entire_ratio = windowed_source_size_pxl / entire_source_size_pxl;

                // how many times should we repeat the sampled image?
                let window_to_entire_multi = target_size_pxl / windowed_source_size_pxl;

                // okay here we go
                // start with figuring out where in the square we should sample for
                let target_coords_txl1 = fract(tex_coords * window_to_entire_multi);
                // now figure out where in the square we should sample, this will just zoom in
                let target_coords_txl = target_coords_txl1 * window_to_entire_ratio + windowed_source_offset_txl;

                let result: vec4<f32> = textureSample(tex, tex_sampler, target_coords_txl);
                "###;
            )
        };

        // let _uniforms = BasicUniform::from_dims(target_dims);

        let conf = GraphicsCreator::default()
            .with_first_texture_format(DEFAULT_LOADED_TEXTURE_FORMAT)
            .with_dst_format(DEFAULT_TEXTURE_FORMAT)
            .with_mag_filter(wgpu::FilterMode::Nearest)
            .with_address_mode(address_mode);

        let graphics = GraphicsRefCustom::new_with_src(
            name,
            c, // gets dims from here
            &repeat_img,
            &conf,
            GraphicsAssets::LocalFilesystem(src_path.to_path_buf()),
        );
        graphics.update_uniforms_other(
            c,
            [0.0, 0.0, 0.0, 0.0],
            [
                source_dims[0] as f32,
                source_dims[1] as f32,
                target_dims[0] as f32,
                target_dims[1] as f32,
            ],
        );

        Self {
            name: name.to_owned(),
            graphics,
        }
    }

    // for colormaps!
    pub fn new_nearest(name: &str, src_path: &PathBuf, c: &GraphicsWindowConf) -> Self {
        // let _source_dims = wgpu::Texture::from_path(c.window, src_path).unwrap().size();
        // let target_dims = c.dims;
        let repeat_img: String = build_shader! {
            (
                raw r###"


                let result: vec4<f32> = textureSample(tex, tex_sampler, tex_coords);
                "###;
            )
        };

        // let uniforms = BasicUniform::from_dims(target_dims);
        let conf = GraphicsCreator::default()
            .with_mag_filter(wgpu::FilterMode::Nearest)
            .with_address_mode(wgpu::AddressMode::ClampToEdge);

        let graphics = GraphicsRefCustom::new_with_src(
            name,
            c,
            &repeat_img,
            &conf,
            GraphicsAssets::LocalFilesystem(src_path.to_path_buf()),
        );
        Self {
            name: name.to_owned(),
            graphics,
        }
    }

    pub fn update_uniforms(&self, c: &GraphicsWindowConf, offset: Vec2, size: Vec2) {
        self.graphics
            .update_uniforms(c, [offset.x, offset.y, size.x, size.y]);
    }
}
