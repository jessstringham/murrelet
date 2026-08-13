// Shared headless-entry macros (PLAN-backend-switch step 3).
//
// The 2 macros below (`headless_svg!`, `headless_png!`) are the lifted,
// harness-agnostic versions of leaves' internal `sketch_main!(@headless_*)` arms.
// They drive the headless entry-loop for any `HeadlessHarness` implementor:
// leaves' `LeavesHarness<C, CC>` today, sibling repos' own config-loading types
// tomorrow.
//
// Each macro takes `$harness:ty + $build:expr` and, for one resolved job, builds
// the harness with per-job overrides, asks the harness for dims / capture path /
// svg-save config, loops N frames (the harness's `earlystop()`, default 1 =
// one-shot) of tick / prepare / render to let stateful sketches settle, then
// captures once at the end. (`headless_png!` preserves the BUG-L349 fix shape:
// loop tick→prepare→render_passes per frame, capture once.) `earlystop` is a
// runtime knob (the CLI `--earlystop` flag, default 1 = one-shot), not a
// compile-time flag.
//
// SVG arms call `::murrelet_svg::headless::render_to_svg` via absolute path —
// the call site's crate is expected to have `murrelet_svg` as a transitive
// dep (it already does via `leave_prelude` for leaves callers). This avoids
// pulling `murrelet_svg` into `murrelet_gpu`'s dep graph.

/// Headless SVG render body (config-driven, no window). Loops over jobs() — one
/// job normally, many under --batch. Build closure shape: `fn(&Conf) -> Model`,
/// where `Model: ToMixedDrawables + IsMurreletModel`.
///
/// First ticks the model's update loop to frame N (the harness's `earlystop()`,
/// default 1 = one-shot) so a STATEFUL Drawing reaches its accumulated/settled
/// state before drawing. Mirrors the web/wasm `tick`: advance the harness one
/// frame, push the fresh config into the model, then tick the model's own state.
/// Requires the model to be `IsMurreletModel` (`set_conf` + `update_with_world`)
/// as well as `ToMixedDrawables`; stateless sketches just run `earlystop` (=1)
/// frames of a no-op-ish update then render. Assumes the `TopLevelLiveCode`
/// `.drawing` convention. Both bounds enforced via static `_assert_*` helpers so
/// a wrong-arm pick errors at the macro invocation site, not deep inside
/// `set_conf` / `render_to_svg`.
#[macro_export]
macro_rules! headless_svg {
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
                .unwrap_or(1);
            let mut __frame_ms: Vec<f64> = Vec::new();
            for frame in 0..earlystop {
                let __t = ::std::time::Instant::now();
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
                __frame_ms.push(__t.elapsed().as_secs_f64() * 1000.0);
            }
            let svg_conf = match job.output {
                Some(out) => <$harness as $crate::HeadlessHarness>::default_svg_path(&harness)
                    .with_capture_path(out),
                None => <$harness as $crate::HeadlessHarness>::default_svg_path(&harness),
            };
            let __tr = ::std::time::Instant::now();
            ::murrelet_svg::headless::render_to_svg(&model, &svg_conf);
            let __render_ms = __tr.elapsed().as_secs_f64() * 1000.0;
            // Per-frame settle-loop speed + the one-time final render. BUG-L377.
            let __mean = if __frame_ms.is_empty() { 0.0 }
                else { __frame_ms.iter().sum::<f64>() / __frame_ms.len() as f64 };
            let __list: Vec<String> = __frame_ms.iter().map(|m| format!("{:.3}", *m)).collect();
            println!(
                "HEADLESS_TIMING {{\"frames\":{},\"frame_ms_mean\":{:.3},\"frame_ms\":[{}],\"render_ms\":{:.3}}}",
                __frame_ms.len(), __mean, __list.join(","), __render_ms
            );
        }
    }};
}

/// Headless PNG render body (gpu pipeline, off-screen, no window). The device
/// is built once; jobs() loops (one job normally, many under --batch). Render
/// size: per-job `--resolution`, else `harness.default_dims()` — matching the
/// windowed path's `GraphicsWindowConf`. Build closure shape:
/// `fn(&GraphicsWindowConf, &Conf) -> Graphic`, where `Graphic: IsHeadlessGraphic`.
///
/// Loops the full `tick → prepare → render_passes` triple N times (the harness's
/// `earlystop()`, default 1 = one-shot) so a STATEFUL gpu sketch settles before
/// the final capture. Running `render_passes` per frame is what lets GPU feedback
/// (e.g. `res_feedback`) accumulate — windowed runs do the same. `tick()` and
/// `prepare()` default to no-op, so stateless gpu sketches behave like a single
/// render plus (at default earlystop=1) just one pass. One readback at the end.
/// `IsHeadlessGraphic` bound enforced via a static
/// `_assert_is_headless_graphic(&graphic)` so a wrong-arm pick errors at the
/// macro invocation site, not at `.prepare()` / `render_headless_graphic_passes`.
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
            let earlystop = <$harness as $crate::HeadlessHarness>::earlystop(&harness)
                .unwrap_or(1);
            let mut __frame_ms: Vec<f64> = Vec::new();
            let mut __display = None;
            for _ in 0..earlystop {
                let __t = ::std::time::Instant::now();
                graphic.tick(
                    &c,
                    <$harness as $crate::HeadlessHarness>::world(&harness),
                );
                graphic.prepare(&c);
                __display =
                    Some($crate::headless::render_headless_graphic_passes(&owned, &c, &graphic));
                __frame_ms.push(__t.elapsed().as_secs_f64() * 1000.0);
            }
            let out = job
                .output
                .or_else(|| <$harness as $crate::HeadlessHarness>::default_png_path(&harness))
                .expect("no png save path configured");
            if let Some(parent) = out.parent() {
                let _ = ::std::fs::create_dir_all(parent);
            }
            let __display = __display.expect("stateful headless render ran zero frames");
            let __tc = ::std::time::Instant::now();
            $crate::headless::capture_display_to_png(&c, &__display, &out)
                .expect("headless png capture failed");
            let __capture_ms = __tc.elapsed().as_secs_f64() * 1000.0;
            // Per-frame settle-loop speed + the one-time final readback. BUG-L377.
            let __mean = if __frame_ms.is_empty() { 0.0 }
                else { __frame_ms.iter().sum::<f64>() / __frame_ms.len() as f64 };
            let __list: Vec<String> = __frame_ms.iter().map(|m| format!("{:.3}", *m)).collect();
            println!(
                "HEADLESS_TIMING {{\"frames\":{},\"frame_ms_mean\":{:.3},\"frame_ms\":[{}],\"capture_ms\":{:.3}}}",
                __frame_ms.len(), __mean, __list.join(","), __capture_ms
            );
        }
    }};
}
