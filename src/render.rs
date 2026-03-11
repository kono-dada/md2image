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
use image::{DynamicImage, GenericImage, RgbaImage};
use tempfile::TempDir;
use url::Url;

use crate::error::{AppError, Result};
use crate::katex;

const MAX_CAPTURE_SLICE_PX_HEIGHT: u32 = 4_096;

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
        katex::stage_assets(temp_dir.path())?;
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
        let viewport_height = max_capture_slice_css_height(capture_scale)?.min(content_height);
        let step_started = Instant::now();
        set_viewport(&tab, options.width, viewport_height, capture_scale)?;
        log_timing(options.timing, "set viewport", step_started);

        let step_started = Instant::now();
        wait_for_layout(&tab)?;
        log_timing(options.timing, "layout wait #2", step_started);

        let step_started = Instant::now();
        let png = capture_document_png(
            &tab,
            options.width,
            content_height,
            viewport_height,
            options.scale,
            capture_scale,
        )?;
        log_timing(options.timing, "capture screenshot", step_started);

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
    tab.wait_for_element("body").map_err(map_browser_error)?;

    tab.evaluate(
        r#"
            new Promise((resolve, reject) => {
                const deadline = Date.now() + 5000;

                const waitForMath = () => {
                    const status = window.__md2imageMathStatus;

                    if (!status || status.done === true) {
                        if (status && status.ok === false) {
                            reject(new Error(`MD2IMAGE_MATH_ERROR:${status.error || "unknown KaTeX error"}`));
                            return;
                        }

                        Promise.resolve(document.fonts ? document.fonts.ready : null)
                            .then(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))))
                            .then(resolve, reject);
                        return;
                    }

                    if (Date.now() >= deadline) {
                        reject(new Error("timed out waiting for KaTeX rendering"));
                        return;
                    }

                    requestAnimationFrame(waitForMath);
                };

                waitForMath();
            });
        "#,
        true,
    )
    .map_err(map_browser_error)?;

    thread::sleep(Duration::from_millis(50));

    Ok(())
}

fn measure_content_height(tab: &Arc<headless_chrome::Tab>) -> Result<u32> {
    let result = tab
        .evaluate(
            r#"
                (() => {
                    const body = document.body;

                    if (!body) {
                        return 1;
                    }

                    const bodyStyle = getComputedStyle(body);
                    const paddingTop = parseFloat(bodyStyle.paddingTop || "0") || 0;
                    const paddingBottom = parseFloat(bodyStyle.paddingBottom || "0") || 0;
                    const contentBottom = Array.from(body.children).reduce((maxBottom, element) => {
                        const rect = element.getBoundingClientRect();
                        return Math.max(maxBottom, rect.bottom + window.scrollY);
                    }, paddingTop);

                    return Math.ceil(Math.max(1, contentBottom + paddingBottom));
                })()
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

    u32::try_from(height).map_err(|_| {
        AppError::Render(format!(
            "content height exceeds the supported range: {height}px"
        ))
    })
}

fn max_capture_slice_css_height(capture_scale: f64) -> Result<u32> {
    if !capture_scale.is_finite() || capture_scale <= 0.0 {
        return Err(AppError::Render(format!(
            "capture scale must be a positive finite number, got {capture_scale}"
        )));
    }

    let slice_height = (f64::from(MAX_CAPTURE_SLICE_PX_HEIGHT) / capture_scale)
        .floor()
        .max(1.0);

    if !slice_height.is_finite() || slice_height > f64::from(u32::MAX) {
        return Err(AppError::Render(format!(
            "capture slice height is out of range for scale {capture_scale}"
        )));
    }

    Ok(slice_height as u32)
}

fn capture_document_png(
    tab: &Arc<headless_chrome::Tab>,
    css_width: u32,
    css_height: u32,
    viewport_height: u32,
    target_scale: f64,
    capture_scale: f64,
) -> Result<Vec<u8>> {
    if css_height <= viewport_height && (capture_scale - target_scale).abs() < f64::EPSILON {
        return tab
            .capture_screenshot(Page::CaptureScreenshotFormatOption::Png, None, None, true)
            .map_err(|error| AppError::Render(error.to_string()));
    }

    let target_width = scaled_dimension(css_width, target_scale)?;
    let target_height = scaled_dimension(css_height, target_scale)?;
    let mut stitched = RgbaImage::new(target_width, target_height);

    let mut offset_css = 0;
    let mut current_viewport_height = viewport_height;

    while offset_css < css_height {
        let remaining_css = css_height - offset_css;
        let slice_css_height = remaining_css.min(viewport_height);

        if slice_css_height != current_viewport_height {
            set_viewport(tab, css_width, slice_css_height, capture_scale)?;
            wait_for_paint(tab)?;
            current_viewport_height = slice_css_height;
        }

        scroll_to(tab, offset_css)?;
        wait_for_paint(tab)?;

        let png = tab
            .capture_screenshot(Page::CaptureScreenshotFormatOption::Png, None, None, true)
            .map_err(|error| AppError::Render(error.to_string()))?;
        let mut slice = decode_png_rgba(&png)?;

        let top_px = scaled_coordinate(offset_css, target_scale)?;
        let bottom_px = scaled_coordinate(offset_css + slice_css_height, target_scale)?;
        let expected_slice_height = bottom_px - top_px;

        if slice.width() != target_width || slice.height() != expected_slice_height {
            slice = DynamicImage::ImageRgba8(slice)
                .resize_exact(target_width, expected_slice_height, FilterType::Lanczos3)
                .into_rgba8();
        }

        stitched.copy_from(&slice, 0, top_px).map_err(|error| {
            AppError::Render(format!(
                "failed to stitch screenshot slice at y={offset_css}: {error}"
            ))
        })?;

        offset_css += slice_css_height;
    }

    encode_png(&DynamicImage::ImageRgba8(stitched))
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

fn scroll_to(tab: &Arc<headless_chrome::Tab>, y: u32) -> Result<()> {
    tab.evaluate(
        &format!(
            r#"
                (() => {{
                    window.scrollTo(0, {});
                    return window.scrollY;
                }})()
            "#,
            y
        ),
        false,
    )
    .map_err(|error| AppError::Render(error.to_string()))?;

    Ok(())
}

fn wait_for_paint(tab: &Arc<headless_chrome::Tab>) -> Result<()> {
    tab.evaluate(
        r#"
            new Promise(resolve => {
                requestAnimationFrame(() => requestAnimationFrame(resolve));
            });
        "#,
        true,
    )
    .map_err(|error| AppError::Render(error.to_string()))?;

    thread::sleep(Duration::from_millis(25));

    Ok(())
}

fn decode_png_rgba(png: &[u8]) -> Result<RgbaImage> {
    Ok(image::load_from_memory_with_format(png, ImageFormat::Png)
        .map_err(|error| AppError::Render(format!("failed to decode screenshot PNG: {error}")))?
        .into_rgba8())
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| AppError::Render(format!("failed to encode PNG: {error}")))?;
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

fn scaled_coordinate(value: u32, multiplier: f64) -> Result<u32> {
    let scaled = (f64::from(value) * multiplier).round();
    if !scaled.is_finite() || scaled < 0.0 || scaled > f64::from(u32::MAX) {
        return Err(AppError::Render(format!(
            "scaled coordinate is out of range: {value} * {multiplier}"
        )));
    }

    Ok(scaled as u32)
}

fn map_browser_error(error: impl ToString) -> AppError {
    const PREFIX: &str = "MD2IMAGE_MATH_ERROR:";

    let message = error.to_string();
    if let Some(start) = message.find(PREFIX) {
        let detail = message[start + PREFIX.len()..]
            .trim()
            .trim_end_matches('}')
            .trim();
        return AppError::MathRender(detail.to_string());
    }

    AppError::Render(message)
}

fn log_timing(enabled: bool, label: &str, started_at: Instant) {
    if enabled {
        eprintln!(
            "  render/{label:>16}: {:.1} ms",
            started_at.elapsed().as_secs_f64() * 1000.0
        );
    }
}
