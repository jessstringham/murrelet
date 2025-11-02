use std::{path::PathBuf, str::FromStr};

use clap::Parser;

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
            width: 1080,
            height: 1080,
            // width: 2000,
            // height: 2000,
            // width: 2000,
            // height: 2000,
            // width: 750,
            // height: 750,
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, allow_hyphen_values = true)]
pub struct BaseConfigArgs {
    pub config_path: PathBuf,
    pub template_path: PathBuf, // todo, i probably should drop this
    #[arg(long, help = "record video")]
    pub capture: bool,

    #[arg(short, long, default_value_t = Default::default())]
    pub resolution: TextureDimensions, // window resolution
    #[arg(long, default_value_t = 4, value_parser = clap::value_parser!(u32).range(1..=8))]
    pub texture_multiplier: u32, // controls number of pixels the shaders work on

    #[arg(long)]
    pub earlystop: Option<u64>,

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
