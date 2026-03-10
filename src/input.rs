use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::Path;

use crate::error::{AppError, Result};

pub fn read_markdown(input: Option<&Path>) -> Result<String> {
    match input {
        Some(path) => fs::read_to_string(path).map_err(|source| AppError::ReadFile {
            path: path.to_path_buf(),
            source,
        }),
        None => read_stdin_if_available(),
    }
}

fn read_stdin_if_available() -> Result<String> {
    if io::stdin().is_terminal() {
        return Err(no_input_error());
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(AppError::ReadStdin)?;

    if buffer.trim().is_empty() {
        return Err(no_input_error());
    }

    Ok(buffer)
}

fn no_input_error() -> AppError {
    AppError::Usage {
        message:
            "no input provided. Pass `INPUT` or pipe Markdown into stdin.\n\nUsage: md2image [INPUT] -o <PATH>"
                .to_string(),
    }
}
