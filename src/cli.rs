use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "md2image",
    version,
    about = "Render Markdown to a PNG with Chrome/Chromium"
)]
pub struct Cli {
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    #[arg(
        short,
        long,
        value_name = "PATH",
        required_unless_present = "stdout",
        conflicts_with = "stdout"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        long,
        required_unless_present = "output",
        conflicts_with = "output",
        help = "Write the PNG bytes to stdout instead of a file"
    )]
    pub stdout: bool,

    #[arg(long, default_value_t = 960, value_name = "PX")]
    pub width: u32,

    #[arg(long, default_value_t = 1.0, value_name = "MULTIPLIER")]
    pub scale: f64,

    #[arg(long, default_value_t = 1.0, value_name = "MULTIPLIER")]
    pub supersample: f64,

    #[arg(long, default_value_t = false)]
    pub timing: bool,

    #[arg(long, default_value = "default", value_name = "NAME")]
    pub theme: String,

    #[arg(long, value_name = "PATH")]
    pub browser: Option<PathBuf>,
}
