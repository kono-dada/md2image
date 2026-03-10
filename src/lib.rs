pub mod app;
pub mod browser;
pub mod cli;
pub mod error;
pub mod html;
pub mod input;
mod katex;
pub mod markdown;
pub mod render;

pub use app::run;
pub use cli::Cli;
pub use error::{AppError, ExitCode, Result};
