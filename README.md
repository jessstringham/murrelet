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

# How expressions work

*this is a work in progress and is probably pretty sloppy with programming language terms sorry*

## State scope

Some interesting variables are injected in different scopes, making them available in different fields.

In a basic example, you need to know about just two scopes:
 - world: the context per frame. includes things like time, midi, audio, global functions, and the *app > ctx* field. You can use these variables in every field (except the time config).
 - unitcell: the context per unitcell, which includes information like the x and y location and a unique seed for each instance.


### Detailed breakdown

That's an oversimplification. Here's roughly how the scopes should work:

These three do strictly build on top of each other
* program-level: these are functions and variables set for every frame. It is hidden away in `LiveCodeUtil`, so you probably won't run into it.
-  timeless: same as World but excludes the `t`-based variable. Basically exclusively used to load the `AppConfigTiming` config.
- world: same as above.

Once you're within a world, going deeper can get as complicated as you want using a combination of:

* unitcell: same as above
* lazyeval: These are variables that are evaluated in your sketch itself, which let's you add custom variables specific to the sketch. The config returns an expression you add your additional context to and then evaluate.

For example, you might set up a sketch where a unitcell sequencer might contain a second unitcell sequencer (using a different variable prefix) that draws things that combine the outer and inner unitcell's variables.


## Expression variables

To see how exactly the variables are defined, you generally want to look for the `IsLivecodeSrc` trait implementation.

## Timing

The float variable `t` represents time in expressions. This is very useful for making things bounce and change to a bpm for live performances. I also use it to explore parameter spaces, like setting a field to `s(ease(t, 0.25), 1.0, 20.0)` to ease between 1.0 and 20.0.

The value of `t` is an abstraction that should increment by `1.0` every bar, given the definitions in the fields of `AppConfigTiming`, which might look something like this:

```yaml
app:
  ...
  time:
    realtime: true
	fps: 60.0
	bpm: 135.0
	beats_per_bar: 4.0 # defaults to 4.0
```


 ### The realtime flag

For live performances, this should probably be set to `true` so you can match the `bpm` of the music, regardless of if the visuals start rendering faster or slower. For recording a video, you might want `realtime` to be `false` to avoid jumps.

Aside: For generative art, I sometimes switch between them: the glitchiness based on how fast my machine is rendering can make nice textures of *realtime*: `true`, but other times I want the even spacing of *realtime*: `false`.

If  *realtime*: `true`, it'll use *bpm* and *beats_per_bar* and the system's clock to figure out what `t` should be. If *realtime*: `false`, instead of the system time, it'll use the current frame number to compute `t`.
