use std::env;
use std::path::{Path, PathBuf};

use which::which;

use crate::error::{AppError, Result};

const BROWSER_ENV: &str = "MD2IMAGE_BROWSER";

pub fn resolve_browser_path(cli_override: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = cli_override {
        return validate(path.to_path_buf());
    }

    if let Some(path) = env::var_os(BROWSER_ENV) {
        return validate(PathBuf::from(path));
    }

    for candidate in platform_candidates() {
        if candidate.exists() {
            return validate(candidate);
        }
    }

    for binary in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "chrome",
    ] {
        if let Ok(path) = which(binary) {
            return validate(path);
        }
    }

    Err(AppError::BrowserNotFound)
}

fn validate(path: PathBuf) -> Result<PathBuf> {
    if !path.exists() {
        return Err(AppError::BrowserPathMissing { path });
    }

    if !path.is_file() {
        return Err(AppError::BrowserPathInvalid { path });
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(&path)
            .map_err(|_| AppError::BrowserPathInvalid { path: path.clone() })?;

        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(AppError::BrowserPathInvalid { path });
        }
    }

    Ok(path)
}

fn platform_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ));
        candidates.push(PathBuf::from(
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        candidates.push(PathBuf::from("/usr/bin/google-chrome"));
        candidates.push(PathBuf::from("/usr/bin/google-chrome-stable"));
        candidates.push(PathBuf::from("/usr/bin/chromium"));
        candidates.push(PathBuf::from("/usr/bin/chromium-browser"));
    }

    #[cfg(target_os = "windows")]
    {
        candidates.push(PathBuf::from(
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        ));
        candidates.push(PathBuf::from(
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ));
        candidates.push(PathBuf::from(
            r"C:\Program Files\Chromium\Application\chrome.exe",
        ));
    }

    candidates
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn platform_candidates_are_present() {
        let candidates = super::platform_candidates();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn missing_browser_override_is_reported() {
        let error = super::resolve_browser_path(Some(Path::new("/definitely/missing/browser")))
            .expect_err("browser should be missing");

        assert!(matches!(
            error,
            crate::error::AppError::BrowserPathMissing { .. }
        ));
    }
}
