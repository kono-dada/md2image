use std::fs;
use std::path::Path;

use crate::error::{AppError, Result};

pub const STYLESHEET_PATH: &str = "./katex/katex.min.css";
pub const SCRIPT_PATH: &str = "./katex/katex.min.js";

const ASSET_DIR: &str = "katex";

include!(concat!(env!("OUT_DIR"), "/katex_assets.rs"));

pub fn stage_assets(root: &Path) -> Result<()> {
    let asset_root = root.join(ASSET_DIR);

    for (relative_path, bytes) in KATEX_ASSETS {
        let path = asset_root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(AppError::TempDir)?;
        }

        fs::write(&path, bytes).map_err(|error| {
            AppError::Render(format!(
                "failed to write KaTeX asset {}: {error}",
                path.display()
            ))
        })?;
    }

    Ok(())
}
