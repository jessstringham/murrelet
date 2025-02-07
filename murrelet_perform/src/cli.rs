use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, allow_hyphen_values = true)]
pub struct BaseConfigArgs {
    pub config_path: PathBuf,
    pub template_path: PathBuf, // todo, i probably should drop this
    #[arg(long, help = "record video")]
    pub capture: bool,

    #[arg(trailing_var_arg = true)]
    pub sketch_args: Vec<String>,
}
