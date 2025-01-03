use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct BaseConfigArgs {
    pub config_path: PathBuf,
    pub template_path: PathBuf, // todo, i probably should drop this
}
