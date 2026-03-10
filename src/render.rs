use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use headless_chrome::protocol::cdp::{Emulation, Page};
use headless_chrome::{Browser, LaunchOptions};
use image::ImageFormat;
use image::imageops::FilterType;
use tempfile::TempDir;
use url::Url;

use crate::error::{AppError, Result};

pub struct RenderOptions {
    pub width: u32,
    pub scale: f64,
    pub supersample: f64,
    pub timing: bool,
    pub browser_path: PathBuf,
}

pub trait Renderer {
    fn render(&self, html: &str, options: &RenderOptions) -> Result<Vec<u8>>;
}

pub struct ChromiumRenderer;

impl Renderer for ChromiumRenderer {
    fn render(&self, html: &str, options: &RenderOptions) -> Result<Vec<u8>> {
        let temp_dir = TempDir::new().map_err(AppError::TempDir)?;
        let html_path = write_temp_html(temp_dir.path(), html)?;
        let file_url = Url::from_file_path(&html_path).map_err(|_| {
            AppError::Render(format!(
                "failed to convert HTML path to file URL: {}",
                html_path.display()
            ))
        })?;

        let step_started = Instant::now();
        let browser = launch_browser(options, temp_dir.path())?;
        log_timing(options.timing, "launch browser", step_started);

        let step_started = Instant::now();
        let tab = browser.new_tab().map_err(|error| AppError::BrowserLaunch {
            path: options.browser_path.clone(),
            message: error.to_string(),
        })?;
        log_timing(options.timing, "open tab", step_started);

        let step_started = Instant::now();
        tab.navigate_to(file_url.as_str())
            .and_then(|tab| tab.wait_until_navigated())
            .map_err(|error| AppError::Render(error.to_string()))?;
        log_timing(options.timing, "navigate", step_started);

        let step_started = Instant::now();
        wait_for_layout(&tab)?;
        log_timing(options.timing, "layout wait #1", step_started);

        let step_started = Instant::now();
        let content_height = measure_content_height(&tab)?;
        log_timing(options.timing, "measure height", step_started);

        let capture_scale = options.scale * options.supersample;
        let step_started = Instant::now();
        set_viewport(&tab, options.width, content_height, capture_scale)?;
        log_timing(options.timing, "set viewport", step_started);

        let step_started = Instant::now();
        wait_for_layout(&tab)?;
        log_timing(options.timing, "layout wait #2", step_started);

        let step_started = Instant::now();
        let png = tab
            .capture_screenshot(Page::CaptureScreenshotFormatOption::Png, None, None, true)
            .map_err(|error| AppError::Render(error.to_string()))?;
        log_timing(options.timing, "capture screenshot", step_started);

        let step_started = Instant::now();
        let png = finalize_png(
            png,
            options.width,
            content_height,
            options.scale,
            options.supersample,
        )?;
        log_timing(options.timing, "finalize png", step_started);

        Ok(png)
    }
}

pub struct PureRustRenderer;

impl Renderer for PureRustRenderer {
    fn render(&self, _html: &str, _options: &RenderOptions) -> Result<Vec<u8>> {
        Err(AppError::Render(
            "pure Rust rendering is reserved for a future version".to_string(),
        ))
    }
}

fn launch_browser(options: &RenderOptions, profile_root: &Path) -> Result<Browser> {
    let mut args: Vec<&OsStr> = Vec::new();
    args.push(OsStr::new("--headless=new"));
    args.push(OsStr::new("--hide-scrollbars"));
    args.push(OsStr::new("--disable-component-update"));
    args.push(OsStr::new("--no-default-browser-check"));

    let runtime_home = profile_root.join("runtime-home");
    fs::create_dir_all(&runtime_home).map_err(AppError::TempDir)?;

    let mut process_envs = HashMap::new();
    process_envs.insert("HOME".to_string(), runtime_home.display().to_string());
    process_envs.insert("TMPDIR".to_string(), profile_root.display().to_string());

    let launch_options = LaunchOptions::default_builder()
        .headless(false)
        .path(Some(options.browser_path.clone()))
        .window_size(Some((options.width, 900)))
        .sandbox(false)
        .args(args)
        .process_envs(Some(process_envs))
        .user_data_dir(Some(profile_root.join("browser-profile")))
        .build()
        .map_err(|error| AppError::BrowserLaunch {
            path: options.browser_path.clone(),
            message: error.to_string(),
        })?;

    Browser::new(launch_options).map_err(|error| AppError::BrowserLaunch {
        path: options.browser_path.clone(),
        message: error.to_string(),
    })
}

fn write_temp_html(dir: &Path, html: &str) -> Result<PathBuf> {
    let path = dir.join("document.html");
    fs::write(&path, html).map_err(|error| {
        AppError::Render(format!("failed to write temporary HTML document: {error}"))
    })?;
    Ok(path)
}

fn wait_for_layout(tab: &Arc<headless_chrome::Tab>) -> Result<()> {
    tab.wait_for_element("body")
        .map_err(|error| AppError::Render(error.to_string()))?;

    tab.evaluate(
        r#"
            Promise.resolve(document.fonts ? document.fonts.ready : null)
                .then(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
        "#,
        true,
    )
    .map_err(|error| AppError::Render(error.to_string()))?;

    thread::sleep(Duration::from_millis(50));

    Ok(())
}

fn measure_content_height(tab: &Arc<headless_chrome::Tab>) -> Result<u32> {
    let result = tab
        .evaluate(
            r#"
                Math.ceil(
                    Math.max(
                        document.documentElement.scrollHeight,
                        document.body ? document.body.scrollHeight : 0,
                        document.documentElement.offsetHeight,
                        document.body ? document.body.offsetHeight : 0
                    )
                )
            "#,
            false,
        )
        .map_err(|error| AppError::Render(error.to_string()))?;

    let Some(value) = result.value else {
        return Err(AppError::Render(
            "browser did not return a content height".to_string(),
        ));
    };

    let height = value.as_u64().ok_or_else(|| {
        AppError::Render("browser returned a non-numeric content height".to_string())
    })?;

    Ok(height.clamp(1, 60_000) as u32)
}

fn set_viewport(
    tab: &Arc<headless_chrome::Tab>,
    width: u32,
    height: u32,
    scale: f64,
) -> Result<()> {
    tab.call_method(Emulation::SetDeviceMetricsOverride {
        width,
        height,
        device_scale_factor: scale,
        mobile: false,
        scale: None,
        screen_width: Some(width),
        screen_height: Some(height),
        position_x: None,
        position_y: None,
        dont_set_visible_size: None,
        screen_orientation: None,
        viewport: None,
        display_feature: None,
        device_posture: None,
    })
    .map_err(|error| AppError::Render(error.to_string()))?;

    Ok(())
}

fn finalize_png(
    png: Vec<u8>,
    css_width: u32,
    css_height: u32,
    scale: f64,
    supersample: f64,
) -> Result<Vec<u8>> {
    let target_width = scaled_dimension(css_width, scale)?;
    let target_height = scaled_dimension(css_height, scale)?;

    if (supersample - 1.0).abs() < f64::EPSILON {
        return Ok(png);
    }

    let image = image::load_from_memory_with_format(&png, ImageFormat::Png)
        .map_err(|error| AppError::Render(format!("failed to decode screenshot PNG: {error}")))?;
    let resized = image.resize_exact(target_width, target_height, FilterType::Lanczos3);

    let mut output = Cursor::new(Vec::new());
    resized
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| AppError::Render(format!("failed to encode supersampled PNG: {error}")))?;

    Ok(output.into_inner())
}

fn scaled_dimension(value: u32, multiplier: f64) -> Result<u32> {
    let scaled = (f64::from(value) * multiplier).round();
    if !scaled.is_finite() || scaled <= 0.0 || scaled > f64::from(u32::MAX) {
        return Err(AppError::Render(format!(
            "scaled dimension is out of range: {value} * {multiplier}"
        )));
    }

    Ok(scaled as u32)
}

fn log_timing(enabled: bool, label: &str, started_at: Instant) {
    if enabled {
        eprintln!(
            "  render/{label:>16}: {:.1} ms",
            started_at.elapsed().as_secs_f64() * 1000.0
        );
    }
}
