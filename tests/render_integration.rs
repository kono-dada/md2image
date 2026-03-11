use std::path::Path;

use md2image::browser::resolve_browser_path;
use md2image::html::build_html;
use md2image::markdown::parse;
use md2image::render::{ChromiumRenderer, RenderOptions, Renderer};

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let reader = decoder.read_info().expect("valid PNG");
    let info = reader.info();
    (info.width, info.height)
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
    let document = parse(markdown);
    let html = build_html(&document, 960, "default");
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
    let browser_path = match resolve_browser_path(None) {
        Ok(path) => path,
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            return;
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    };

    let markdown = "> [!NOTE] 这里是提示信息";
    let document = parse(markdown);
    let html = build_html(&document, 960, "default");
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
    let document = parse(markdown);
    let html = build_html(&document, 960, "default");
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
    let document = parse(markdown);
    let html = build_html(&document, 960, "default");
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
fn renders_math_fixture_to_png_when_browser_is_available() {
    let browser_path = match resolve_browser_path(None) {
        Ok(path) => path,
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            return;
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    };

    let markdown = include_str!("fixtures/math.md");
    let document = parse(markdown);
    let html = build_html(&document, 960, "default");
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
        .expect("math PNG render should succeed");

    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 960);
    assert!(height > 0);
}

#[test]
fn invalid_math_still_renders_png() {
    let browser_path = match resolve_browser_path(None) {
        Ok(path) => path,
        Err(md2image::AppError::BrowserNotFound) => {
            eprintln!("skipping browser integration test: no local browser found");
            return;
        }
        Err(error) => panic!("unexpected browser resolution error: {error}"),
    };

    let markdown = include_str!("fixtures/invalid_math.md");
    let document = parse(markdown);
    let html = build_html(&document, 960, "default");
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
        .expect("invalid math should still render");

    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
    let (width, height) = png_dimensions(&png);
    assert_eq!(width, 960);
    assert!(height > 0);
}
