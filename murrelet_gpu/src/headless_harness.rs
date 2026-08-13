// Shared headless-entry interface (PLAN-backend-switch step 1).
//
// The internal arms of leaves' `sketch_main!` (@headless_svg, @headless_png) all
// reach into the leaves `LiveCode` type for the same handful of operations:
// enumerate batch jobs, build a fresh livecode with per-job overrides, ask for the
// texture dims / capture path / svg-save config, advance one frame of update (the
// settle loop, length = `earlystop`), and hand back the world.
//
// `HeadlessHarness` is that interface. Promoting it into murrelet (next to
// `IsHeadlessGraphic`) lets the headless entry-loop macros (step 3) live in
// `murrelet_gpu`, with leaves' `LiveCode` (step 2) and sibling repos' own
// config-loading types both implementing the same trait.
use std::path::PathBuf;

use murrelet_livecode::state::LivecodeWorldState;
use murrelet_perform::perform::SvgDrawConfig;

// One resolved headless render: extra config overrides on top of the global `--set`,
// where the file goes, and an optional render size. A single run is one of these;
// `--batch` produces many. (Mirror of `murrelet_perform::cli::HeadlessJob` — kept
// here so the trait surface lives entirely in `murrelet_gpu`, alongside the rest
// of the headless interface, without forcing trait consumers to import `cli`.)
pub struct HeadlessJob {
    pub overrides: Vec<String>,
    pub output: Option<PathBuf>,
    pub resolution: Option<[u32; 2]>,
}

// What the headless entry-loop needs from whatever owns config-loading + CLI args on
// the consumer side. Leaves' `LiveCode` implements it via the existing
// `headless_jobs()`/`args()`/`png_capture_path()`/`svg_save_path()` plumbing; sibling
// repos write their own implementor on their own config-loading type.
pub trait HeadlessHarness: Sized {
    type Conf;

    fn jobs() -> Vec<HeadlessJob>;
    fn build_with_overrides(overrides: &[String]) -> Self;
    fn config(&self) -> &Self::Conf;
    fn default_dims(&self) -> [u32; 2];
    fn default_png_path(&self) -> Option<PathBuf>;
    fn default_svg_path(&self) -> SvgDrawConfig;

    /// CLI `--earlystop` or equivalent. None = caller's default (typically 100).
    fn earlystop(&self) -> Option<u64> { None }

    // For the *_stateful arms — default no-op so non-stateful consumers don't
    // implement them. `update_for_frame` advances the livecode one frame with a
    // minimal `MurreletAppInput`; `world()` exposes the post-update world for
    // downstream `tick(...)` / `update_with_world(...)` calls.
    fn update_for_frame(&mut self, _frame: u64) {}
    fn world(&self) -> &LivecodeWorldState;
}
