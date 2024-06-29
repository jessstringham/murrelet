# Murrelet

*This README is a work in progress!*

The crates here are part of the livecode engine that I've been building and using to make nearly all the art as [this.xor.that](http://thisxorthat.art).

A demo of this (should be) running [here](https://www.thisxorthat.art/live/foolish-guillemot/). The code for creating the WASM for the website is in `examples/foolish-guillemot`, and the main.js is [here](https://gist.github.com/jessstringham/0654a13257f7aff4912affa5df95e36b).

A high-level overview of the software is [published here](https://alpaca.pubpub.org/pub/dpdnf8lw/release/1?readingCollection=1def0192).


## Disclaimer

This code base is kinda like a bunch of weird workshop tools that are held together with duct tape.

 - These libraries are in initial development and the API is not stable.

 - At this time, I'm not sure if I'll accept PRs on this repo. If there is interest, I might be interested in spinning off a more manageable chunk of the code to maintain and document and all that.

 - I'm still learning Rust and computer graphics, so there will be funny weird things.


# What code is included

There are a few major parts:


## livecode macros

### Livecode

The Livecode macros makes it possible to control parameters of a struct
by injecting some info about the world (time, audio, midi, etc), combined
with expressions.

Right now it's built on top of `evalexpr`. It has support for inputs using:

 - evalexpr (expr to combine)
 - audio (audio)
 - midi (using midir)
 - time (some custom code)

experimental
 - clicks (pretty fun to control a sketch with an ipad!)


### UnitCells

Unitcells can be used to dynamically create a list of things.
The number of things and the arrangement (grid, symmetry) is 
controlled by sequencers (see murrelet_draw/sequencers for examples).


### Experimental: NestEdit

This is a way to access/update a value in a nested struct using a string.


### Experimental: Boop

**This is.. not working. But I haven't wanted to delete the code yet nor have had a reason to get it to work, so it's broken for now. I do want to fix it or reimagine eventually!**

Boop is a funny not-quite-working bit of code that's meant to help interpolate values and avoid hard jumps when you update a value.

The one implemented right now are ODEs, which let you to use some features from animation, like anticipation 
(going a little in the opposite direction before going in the intended direction).


## GPU

There are a few macros here for building shaders.

The `build_shader` just hides some boilerplate of the fragment shader.

```rust
let gradient_def: String = build_shader! {
    (
        raw r###"
        let start: vec4<f32> = uniforms.more_info;
        let end: vec4<f32> = uniforms.more_info_other;
        let result = mix(start, end, tex_coords.x);
        "###;
    )
};
let gradient_red = prebuilt_graphics::new_shader_basic(c, "grad", &gradient_def);
gradient_red
    .update_uniforms_other_tuple(c, ([0.0, 0.0, 1.0, 0.04], [1.0, 0.0, 1.0, 0.04]));
```

and then `build_shader_pipeline` let's you take those graphics and
write to input textures of others.
(for extra fun, I use Fira font with arrow glyphs)

```rust
let example_pipeline = build_shader_pipeline! {
    gradient_red -> drawing_placeholder;
    drawing_placeholder -> DISPLAY;
};
```