pub mod compute;
pub mod device_state;
pub mod editable_shaders;
pub mod gpu_livecode;
pub mod gpu_macros;
pub mod graphics_ref;
pub mod headless;
pub mod headless_harness;
pub mod headless_macros;
pub mod shader_str;
pub mod uniforms;
pub mod window;

pub use headless_harness::{HeadlessHarness, HeadlessJob};
// Re-export the trait the svg-arm macros static-assert on, so sibling
// consumers (catscradle / spoonbill / mockingbird / …) don't have to add
// `murrelet_draw` to their own Cargo.toml just to satisfy the bound check.
pub use murrelet_draw::drawable::ToMixedDrawables;
pub use murrelet_gpu_derive::*;
