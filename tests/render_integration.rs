use std::path::{Path, PathBuf};

use md2image::browser::resolve_browser_path;
use md2image::html::build_html;
use md2image::markdown::render_html;
use md2image::render::{ChromiumRenderer, RenderOptions, Renderer};

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let reader = decoder.read_info().expect("valid PNG");
    let info = reader.info();
    (info.width, info.height)
}

fn browser_path_or_skip() -> Option<PathBuf> {
    match resolve_browser_path(None) {
        Ok(path) => Some(path),
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            None
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    }
}

#[test]
fn missing_browser_override_returns_clear_error() {
    let error = resolve_browser_path(Some(Path::new("/definitely/missing/browser")))
        .expect_err("expected missing browser");

    assert!(matches!(
        error,
        md2image::AppError::BrowserPathMissing { .. }
    ));
}

#[test]
fn renders_fixture_to_png_when_browser_is_available() {
    let browser_path = match resolve_browser_path(None) {
        Ok(path) => path,
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            return;
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    };

    let markdown = include_str!("fixtures/basic.md");
    let rendered = render_html(markdown);
    let html = build_html(&rendered, 960, "default").expect("theme should be valid");
    let renderer = ChromiumRenderer;
    let png = renderer
        .render(
            &html,
            &RenderOptions {
                width: 960,
                scale: 1.0,
                supersample: 1.0,
                timing: false,
                browser_path,
            },
        )
        .expect("PNG render should succeed");

    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 960);
    assert!(height > 0);
}

#[test]
fn short_markdown_does_not_expand_to_viewport_height() {
    let Some(browser_path) = browser_path_or_skip() else {
        return;
    };

    let markdown = "> 这里是提示信息";
    let rendered = render_html(markdown);
    let html = build_html(&rendered, 960, "default").expect("theme should be valid");
    let renderer = ChromiumRenderer;
    let png = renderer
        .render(
            &html,
            &RenderOptions {
                width: 960,
                scale: 1.0,
                supersample: 1.0,
                timing: false,
                browser_path,
            },
        )
        .expect("short markdown PNG render should succeed");

    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 960);
    assert!(
        height < 700,
        "short content should not be stretched to viewport height: got {height}px"
    );
}

#[test]
fn very_tall_markdown_is_rendered_without_truncation() {
    let Some(browser_path) = browser_path_or_skip() else {
        return;
    };

    let markdown = (0..400)
        .map(|index| {
            format!(
                "第{}段：这是一段专门用于制造超长页面的测试文本，它会在较窄的版心里自动换行，从而把整张图片拉得足够高，用来验证分段截图和最终拼接不会截断底部内容。",
                index + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let rendered = render_html(&markdown);
    let html = build_html(&rendered, 400, "default").expect("theme should be valid");
    let renderer = ChromiumRenderer;
    let png = renderer
        .render(
            &html,
            &RenderOptions {
                width: 400,
                scale: 1.0,
                supersample: 1.0,
                timing: false,
                browser_path,
            },
        )
        .expect("very tall markdown PNG render should succeed");

    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 400);
    assert!(
        height > 60_000,
        "very tall markdown should extend beyond the old truncation ceiling: got {height}px"
    );
}

#[test]
fn very_tall_markdown_with_supersample_renders_successfully() {
    let Some(browser_path) = browser_path_or_skip() else {
        return;
    };

    let markdown = (0..220)
        .map(|index| {
            format!(
                "第{}段：这是一段用于验证超长内容在 supersample 模式下也能稳定渲染的测试文本。它会重复很多次，确保渲染流程必须走切片、缩放和拼接，而不是一次性整图处理。",
                index + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let rendered = render_html(&markdown);
    let html = build_html(&rendered, 360, "default").expect("theme should be valid");
    let renderer = ChromiumRenderer;
    let png = renderer
        .render(
            &html,
            &RenderOptions {
                width: 360,
                scale: 1.0,
                supersample: 2.0,
                timing: false,
                browser_path,
            },
        )
        .expect("very tall markdown with supersample should render");

    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 360);
    assert!(
        height > 10_000,
        "expected a tall rendered image, got {height}px"
    );
}

#[test]
fn scale_increases_output_dimensions() {
    let browser_path = match resolve_browser_path(None) {
        Ok(path) => path,
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            return;
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    };

    let markdown = include_str!("fixtures/basic.md");
    let rendered = render_html(markdown);
    let html = build_html(&rendered, 960, "default").expect("theme should be valid");
    let renderer = ChromiumRenderer;
    let png = renderer
        .render(
            &html,
            &RenderOptions {
                width: 960,
                scale: 2.0,
                supersample: 1.0,
                timing: false,
                browser_path,
            },
        )
        .expect("scaled PNG render should succeed");

    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 1920);
    assert!(height > 0);
}

#[test]
fn supersample_preserves_scaled_output_dimensions() {
    let browser_path = match resolve_browser_path(None) {
        Ok(path) => path,
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            return;
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    };

    let markdown = include_str!("fixtures/basic.md");
    let rendered = render_html(markdown);
    let html = build_html(&rendered, 960, "default").expect("theme should be valid");
    let renderer = ChromiumRenderer;
    let png = renderer
        .render(
            &html,
            &RenderOptions {
                width: 960,
                scale: 2.0,
                supersample: 2.0,
                timing: false,
                browser_path,
            },
        )
        .expect("supersampled PNG render should succeed");

    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 1920);
    assert!(height > 0);
}

#[test]
fn renders_tables_and_task_lists_when_browser_is_available() {
    let browser_path = match resolve_browser_path(None) {
        Ok(path) => path,
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            return;
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    };

    let markdown =
        "| Feature | Status |\n| - | - |\n| tables | yes |\n\n- [x] tasks\n- [ ] pending";
    let rendered = render_html(markdown);
    let html = build_html(&rendered, 960, "pico").expect("theme should be valid");
    let renderer = ChromiumRenderer;
    let png = renderer
        .render(
            &html,
            &RenderOptions {
                width: 960,
                scale: 1.0,
                supersample: 1.0,
                timing: false,
                browser_path,
            },
        )
        .expect("standard markdown features should render");

    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 960);
    assert!(height > 0);
}
