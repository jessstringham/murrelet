// Shared headless-entry macros (PLAN-backend-switch step 3).
//
// The 4 macros below (`headless_svg!`, `headless_svg_stateful!`, `headless_png!`,
// `headless_png_stateful!`) are the lifted, harness-agnostic versions of leaves'
// internal `sketch_main!(@headless_*)` arms. They drive the headless entry-loop
// for any `HeadlessHarness` implementor: leaves' `LeavesHarness<C, CC>` today,
// sibling repos' own config-loading types tomorrow.
//
// Each macro takes `$harness:ty + $build:expr` and, for one resolved job, builds
// the harness with per-job overrides, asks the harness for dims / capture path /
// svg-save config, runs the renderer, and writes out. The `*_stateful` variants
// additionally loop N frames (the harness's `earlystop()`, default 100) of
// tick / prepare / render before the final capture — preserving the BUG-L349
// fix shape for `headless_png_stateful!` (loop tick→prepare→render_passes per
// frame, capture once at the end).
//
// SVG arms call `::murrelet_svg::headless::render_to_svg` via absolute path —
// the call site's crate is expected to have `murrelet_svg` as a transitive
// dep (it already does via `leave_prelude` for leaves callers). This avoids
// pulling `murrelet_svg` into `murrelet_gpu`'s dep graph.

/// Headless SVG render body (config-driven, no window). Loops over jobs() — one
/// job normally, many under --batch. Build closure shape: `fn(&Conf) -> Model`,
/// where `Model: ToMixedDrawables`. The trait bound is enforced via a static
/// `_assert_to_mixed_drawables(&model)` helper so a wrong-arm pick (e.g. a build
/// returning an `IsHeadlessGraphic` instead) errors at the macro invocation site,
/// not deep inside `render_to_svg`.
#[macro_export]
macro_rules! headless_svg {
    ($harness:ty, $build:expr) => {{
        fn _assert_to_mixed_drawables<T: $crate::ToMixedDrawables>(_: &T) {}
        let build: fn(&_) -> _ = $build;
        for job in <$harness as $crate::HeadlessHarness>::jobs() {
            let harness = <$harness as $crate::HeadlessHarness>::build_with_overrides(&job.overrides);
            let model = build(<$harness as $crate::HeadlessHarness>::config(&harness));
            _assert_to_mixed_drawables(&model);
            let svg_conf = match job.output {
                Some(out) => <$harness as $crate::HeadlessHarness>::default_svg_path(&harness)
                    .with_capture_path(out),
                None => <$harness as $crate::HeadlessHarness>::default_svg_path(&harness),
            };
            ::murrelet_svg::headless::render_to_svg(&model, &svg_conf);
        }
    }};
}

/// Like `headless_svg!`, but first ticks the model's update loop to frame N
/// (the harness's `earlystop()`, default 100) so a STATEFUL Drawing reaches its
/// accumulated/settled state before drawing. Mirrors the web/wasm `tick`:
/// advance the harness one frame, push the fresh config into the model, then
/// tick the model's own state. Requires the model to be `IsMurreletModel`
/// (`set_conf` + `update_with_world`); plain `headless_svg!` only needs
/// `ToMixedDrawables`. Assumes the `TopLevelLiveCode` `.drawing` convention.
/// Both bounds enforced via static `_assert_*` helpers so a wrong-arm pick
/// errors at the macro invocation site, not deep inside `set_conf` /
/// `render_to_svg`.
#[macro_export]
macro_rules! headless_svg_stateful {
    ($harness:ty, $build:expr) => {{
        fn _assert_to_mixed_drawables<T: $crate::ToMixedDrawables>(_: &T) {}
        fn _assert_is_murrelet_model<
            C: Clone,
            T: ::murrelet_perform::IsMurreletModel<C>,
        >(_: &T) {}
        let build: fn(&_) -> _ = $build;
        for job in <$harness as $crate::HeadlessHarness>::jobs() {
            let mut harness =
                <$harness as $crate::HeadlessHarness>::build_with_overrides(&job.overrides);
            let mut model = build(<$harness as $crate::HeadlessHarness>::config(&harness));
            _assert_to_mixed_drawables(&model);
            _assert_is_murrelet_model(&model);
            let earlystop = <$harness as $crate::HeadlessHarness>::earlystop(&harness)
                .unwrap_or(100);
            for frame in 0..earlystop {
                let app_input =
                    ::murrelet_common::MurreletAppInput::default_with_frames(frame);
                <$harness as $crate::HeadlessHarness>::update_for_frame(&mut harness, frame);
                model.set_conf(
                    <$harness as $crate::HeadlessHarness>::config(&harness)
                        .drawing
                        .clone(),
                );
                model.update_with_world(
                    &app_input,
                    <$harness as $crate::HeadlessHarness>::world(&harness),
                );
            }
            let svg_conf = match job.output {
                Some(out) => <$harness as $crate::HeadlessHarness>::default_svg_path(&harness)
                    .with_capture_path(out),
                None => <$harness as $crate::HeadlessHarness>::default_svg_path(&harness),
            };
            ::murrelet_svg::headless::render_to_svg(&model, &svg_conf);
        }
    }};
}

/// Headless PNG render body (gpu pipeline, off-screen, no window). The device
/// is built once; jobs() loops (one job normally, many under --batch). Render
/// size: per-job `--resolution`, else `harness.default_dims()` — matching the
/// windowed path's `GraphicsWindowConf`. Build closure shape:
/// `fn(&GraphicsWindowConf, &Conf) -> Graphic`, where `Graphic: IsHeadlessGraphic`.
/// The trait bound is enforced via a static `_assert_is_headless_graphic(&graphic)`
/// so a wrong-arm pick (e.g. a build closure returning a `ToMixedDrawables`
/// model instead) errors at the macro invocation site, not at `.prepare()` /
/// `render_headless_graphic_to_png`.
#[macro_export]
macro_rules! headless_png {
    ($harness:ty, $build:expr) => {{
        fn _assert_is_headless_graphic<T: $crate::headless::IsHeadlessGraphic>(_: &T) {}
        let owned = $crate::headless::new_native_device();
        let c_device = owned.to_borrowed();
        let build: fn(&_, &_) -> _ = $build;
        for job in <$harness as $crate::HeadlessHarness>::jobs() {
            let harness =
                <$harness as $crate::HeadlessHarness>::build_with_overrides(&job.overrides);
            let dims = job
                .resolution
                .unwrap_or_else(|| <$harness as $crate::HeadlessHarness>::default_dims(&harness));
            let c = $crate::window::GraphicsWindowConf::new(
                &c_device,
                dims,
                $crate::device_state::GraphicsAssets::Nothing,
            );
            let mut graphic = build(&c, <$harness as $crate::HeadlessHarness>::config(&harness));
            _assert_is_headless_graphic(&graphic);
            // headless prep hook (default no-op): drawer-fed sketches fill their
            // CPU drawer here, the work the windowed update() does. Pure-shader
            // sketches inherit the no-op, so there's no separate arm to pick.
            graphic.prepare(&c);
            let out = job
                .output
                .or_else(|| <$harness as $crate::HeadlessHarness>::default_png_path(&harness))
                .expect("no png save path configured");
            if let Some(parent) = out.parent() {
                let _ = ::std::fs::create_dir_all(parent);
            }
            $crate::headless::render_headless_graphic_to_png(&owned, &c, &graphic, &out)
                .expect("headless png render failed");
        }
    }};
}

/// Like `headless_png!`, but loops the full `tick → prepare → render_passes`
/// triple N times (the harness's `earlystop()`, default 100) so a STATEFUL gpu
/// sketch settles before the final capture. Running `render_passes` per frame
/// is what lets GPU feedback (e.g. `res_feedback`) accumulate — windowed runs
/// do the same. `tick()` and `prepare()` default to no-op, so stateless gpu
/// sketches behave like `headless_png!` plus a few redundant GPU passes (cheap,
/// no state buildup). One readback at the end. `IsHeadlessGraphic` bound
/// enforced via a static `_assert_is_headless_graphic(&graphic)` so a wrong-arm
/// pick errors at the macro invocation site.
#[macro_export]
macro_rules! headless_png_stateful {
    ($harness:ty, $build:expr) => {{
        fn _assert_is_headless_graphic<T: $crate::headless::IsHeadlessGraphic>(_: &T) {}
        let owned = $crate::headless::new_native_device();
        let c_device = owned.to_borrowed();
        let build: fn(&_, &_) -> _ = $build;
        for job in <$harness as $crate::HeadlessHarness>::jobs() {
            let harness =
                <$harness as $crate::HeadlessHarness>::build_with_overrides(&job.overrides);
            let dims = job
                .resolution
                .unwrap_or_else(|| <$harness as $crate::HeadlessHarness>::default_dims(&harness));
            let c = $crate::window::GraphicsWindowConf::new(
                &c_device,
                dims,
                $crate::device_state::GraphicsAssets::Nothing,
            );
            let mut graphic = build(&c, <$harness as $crate::HeadlessHarness>::config(&harness));
            _assert_is_headless_graphic(&graphic);
            let earlystop = <$harness as $crate::HeadlessHarness>::earlystop(&harness)
                .unwrap_or(100);
            for _ in 0..earlystop {
                graphic.tick(
                    &c,
                    <$harness as $crate::HeadlessHarness>::world(&harness),
                );
                graphic.prepare(&c);
                $crate::headless::render_headless_graphic_passes(&owned, &c, &graphic);
            }
            let out = job
                .output
                .or_else(|| <$harness as $crate::HeadlessHarness>::default_png_path(&harness))
                .expect("no png save path configured");
            if let Some(parent) = out.parent() {
                let _ = ::std::fs::create_dir_all(parent);
            }
            $crate::headless::capture_headless_graphic_to_png(&c, &graphic, &out)
                .expect("headless png capture failed");
        }
    }};
}
