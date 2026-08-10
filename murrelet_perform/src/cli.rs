use std::collections::BTreeMap;
use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Clone, Copy)]
pub struct TextureDimensions {
    pub width: u32,
    pub height: u32,
}

impl TextureDimensions {
    pub fn as_dims(&self) -> [u32; 2] {
        [self.width, self.height]
    }
}

impl FromStr for TextureDimensions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('x').collect();
        if parts.len() != 2 {
            return Err("Dimensions must be in format WIDTHxHEIGHT".to_string());
        }

        let width = parts[0].parse::<u32>().map_err(|_| "Invalid width")?;
        let height = parts[1].parse::<u32>().map_err(|_| "Invalid height")?;

        Ok(TextureDimensions { width, height })
    }
}

impl ToString for TextureDimensions {
    fn to_string(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

impl Default for TextureDimensions {
    fn default() -> Self {
        Self {
            // width: 3000,
            // height: 2000,
            // width: 3840,
            // height: 1646,
            width: 800,
            height: 800,
            // width: 2000,
            // height: 2000,
            // width: 800,
            // height: 800,
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, allow_hyphen_values = true)]
pub struct BaseConfigArgs {
    pub config_path: PathBuf,
    #[arg(long, help = "record video")]
    pub capture: bool,

    #[arg(short, long, default_value_t = Default::default())]
    pub resolution: TextureDimensions, // window resolution
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..=8))]
    pub texture_multiplier: u32, // controls number of pixels the shaders work on

    #[arg(long)]
    pub earlystop: Option<u64>,

    // Override config fields by dotted schema path before parsing, e.g.
    // `--set drawing.filename=bird_001.png`. Repeatable. Lets a batch wrapper
    // vary one field per run without authoring a yaml per input.
    #[arg(long = "set", value_name = "PATH=VALUE")]
    pub overrides: Vec<String>,

    // Headless renders write here instead of the config-derived capture path.
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    // Run many headless jobs in one process; see BatchManifest.
    #[arg(long, value_name = "JOBS.yaml")]
    pub batch: Option<PathBuf>,

    #[arg(trailing_var_arg = true)]
    pub sketch_args: Vec<String>,
}
impl BaseConfigArgs {
    pub fn texture_dims(&self) -> TextureDimensions {
        TextureDimensions {
            width: self.resolution.width * self.texture_multiplier,
            height: self.resolution.height * self.texture_multiplier,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn config_path(&self) -> PathBuf {
        self.config_path.clone()
    }

    #[allow(dead_code)]
    pub(crate) fn should_capture(&self) -> bool {
        self.capture
    }
}

// One resolved headless render: extra config overrides on top of the global
// `--set`, where the file goes, and an optional render size. A single run is
// one of these; `--batch` produces many.
pub struct HeadlessJob {
    pub overrides: Vec<String>,
    pub output: Option<PathBuf>,
    pub resolution: Option<[u32; 2]>,
}

#[derive(Debug, Deserialize)]
struct BatchManifest {
    jobs: Vec<BatchJob>,
}

#[derive(Debug, Deserialize)]
struct BatchJob {
    // field path -> value, same string form as `--set KEY=VALUE`.
    #[serde(default)]
    set: BTreeMap<String, String>,
    output: PathBuf,
    // "WxH"; falls back to --resolution when absent.
    #[serde(default)]
    resolution: Option<String>,
}

/// A run is headless when it either asks for an explicit output path
/// (`--output <PATH>` for one render, `--batch <JOBS.yaml>` for many) or sets
/// the `HEADLESS` env var. The env var is the path-less form: render to the
/// config's own save path with no window (penplot's plotter pipeline runs
/// birdmap_render this way — see testprint.py). The `sketch_main!` arms call
/// this to pick window vs headless.
pub fn is_headless() -> bool {
    if std::env::var_os("HEADLESS").is_some() {
        return true;
    }
    let args = BaseConfigArgs::parse();
    args.output.is_some() || args.batch.is_some()
}

// The headless jobs to run: many from `--batch`, else a single job carrying the
// global `--output`. Global `--set` is applied separately (in new_with_overrides),
// so a single job needs no extra overrides.
pub fn headless_jobs() -> Vec<HeadlessJob> {
    let args = BaseConfigArgs::parse();
    let Some(batch_path) = &args.batch else {
        return vec![HeadlessJob {
            overrides: Vec::new(),
            output: args.output.clone(),
            resolution: None,
        }];
    };

    let text = std::fs::read_to_string(batch_path)
        .unwrap_or_else(|e| panic!("could not read --batch manifest {}: {e}", batch_path.display()));
    let manifest: BatchManifest = serde_yaml::from_str(&text)
        .unwrap_or_else(|e| panic!("invalid --batch manifest {}: {e}", batch_path.display()));

    manifest
        .jobs
        .into_iter()
        .map(|job| {
            let overrides = job.set.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
            let resolution = job.resolution.as_deref().map(|s| {
                TextureDimensions::from_str(s)
                    .unwrap_or_else(|e| panic!("bad resolution '{s}' in --batch manifest: {e}"))
                    .as_dims()
            });
            HeadlessJob {
                overrides,
                output: Some(job.output),
                resolution,
            }
        })
        .collect()
}
