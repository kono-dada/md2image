use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Runtime = 1,
    Usage = 2,
}

impl From<ExitCode> for i32 {
    fn from(value: ExitCode) -> Self {
        value as i32
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    Usage { message: String },

    #[error("failed to read markdown file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read markdown from stdin: {0}")]
    ReadStdin(#[source] std::io::Error),

    #[error("unsupported theme `{theme}`. Only `default` is available in v1.")]
    UnsupportedTheme { theme: String },

    #[error("failed to create output directory {path}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write PNG to {path}: {source}")]
    WriteOutput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write PNG to stdout: {0}")]
    WriteStdout(#[source] std::io::Error),

    #[error("failed to render markdown: {0}")]
    Render(String),

    #[error("failed to render LaTeX formula: {0}")]
    MathRender(String),

    #[error(
        "could not find Chrome/Chromium. Install Chrome/Chromium, or pass `--browser <PATH>`, or set `MD2IMAGE_BROWSER`."
    )]
    BrowserNotFound,

    #[error("browser path does not exist: {path}")]
    BrowserPathMissing { path: PathBuf },

    #[error("browser path is not executable: {path}")]
    BrowserPathInvalid { path: PathBuf },

    #[error("failed to launch Chrome/Chromium at {path}: {message}")]
    BrowserLaunch { path: PathBuf, message: String },

    #[error("failed to create temporary render workspace: {0}")]
    TempDir(#[source] std::io::Error),
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Usage { .. } => ExitCode::Usage,
            _ => ExitCode::Runtime,
        }
    }
}
