use std::time::Duration;

use assert_cmd::cargo::cargo_bin_cmd;
use md2image::browser::resolve_browser_path;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn rejects_missing_input() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("out.png");

    cargo_bin_cmd!("md2image")
        .arg("-o")
        .arg(&output)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("no input provided"));
}

#[test]
fn rejects_missing_output_and_stdout() {
    cargo_bin_cmd!("md2image")
        .arg("tests/fixtures/basic.md")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--output <PATH>"))
        .stderr(predicate::str::contains("--stdout"));
}

#[test]
fn rejects_output_with_stdout() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("out.png");

    cargo_bin_cmd!("md2image")
        .arg("tests/fixtures/basic.md")
        .arg("-o")
        .arg(&output)
        .arg("--stdout")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn rejects_unknown_theme() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("out.png");

    cargo_bin_cmd!("md2image")
        .arg("tests/fixtures/basic.md")
        .arg("-o")
        .arg(&output)
        .arg("--theme")
        .arg("dark")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unsupported theme"));
}

#[test]
fn rejects_invalid_scale() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("out.png");

    cargo_bin_cmd!("md2image")
        .arg("tests/fixtures/basic.md")
        .arg("-o")
        .arg(&output)
        .arg("--scale")
        .arg("0")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "`--scale` must be a finite number greater than or equal to 1",
        ));
}

#[test]
fn rejects_invalid_supersample() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("out.png");

    cargo_bin_cmd!("md2image")
        .arg("tests/fixtures/basic.md")
        .arg("-o")
        .arg(&output)
        .arg("--supersample")
        .arg("0")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "`--supersample` must be a finite number greater than or equal to 1",
        ));
}

#[test]
fn input_file_wins_over_stdin() {
    let Some(browser_path) = browser_path_for_test() else {
        eprintln!("skipping browser-dependent CLI test: no local browser found");
        return;
    };

    let temp = tempdir().unwrap();
    let output = temp.path().join("out.png");

    cargo_bin_cmd!("md2image")
        .timeout(Duration::from_secs(30))
        .env("MD2IMAGE_BROWSER", &browser_path)
        .arg("tests/fixtures/basic.md")
        .arg("-o")
        .arg(&output)
        .write_stdin("stdin content should be ignored")
        .assert()
        .success();

    assert!(output.exists());
}

#[test]
fn renders_from_stdin() {
    let Some(browser_path) = browser_path_for_test() else {
        eprintln!("skipping browser-dependent CLI test: no local browser found");
        return;
    };

    let temp = tempdir().unwrap();
    let output = temp.path().join("stdin.png");

    cargo_bin_cmd!("md2image")
        .timeout(Duration::from_secs(30))
        .env("MD2IMAGE_BROWSER", &browser_path)
        .arg("-o")
        .arg(&output)
        .arg("--scale")
        .arg("2")
        .write_stdin("# stdin\n\nhello")
        .assert()
        .success();

    assert!(output.exists());
}

#[test]
fn renders_png_to_stdout() {
    let Some(browser_path) = browser_path_for_test() else {
        eprintln!("skipping browser-dependent CLI test: no local browser found");
        return;
    };

    let assert = cargo_bin_cmd!("md2image")
        .timeout(Duration::from_secs(30))
        .env("MD2IMAGE_BROWSER", &browser_path)
        .arg("tests/fixtures/basic.md")
        .arg("--stdout")
        .assert()
        .success();

    assert!(
        assert
            .get_output()
            .stdout
            .starts_with(&[137, 80, 78, 71, 13, 10, 26, 10])
    );
}

#[test]
fn renders_markdown_with_math() {
    let Some(browser_path) = browser_path_for_test() else {
        eprintln!("skipping browser-dependent CLI test: no local browser found");
        return;
    };

    let temp = tempdir().unwrap();
    let output = temp.path().join("math.png");

    cargo_bin_cmd!("md2image")
        .timeout(Duration::from_secs(30))
        .env("MD2IMAGE_BROWSER", &browser_path)
        .arg("tests/fixtures/math.md")
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    assert!(output.exists());
}

#[test]
fn keeps_rendering_when_math_is_invalid() {
    let Some(browser_path) = browser_path_for_test() else {
        eprintln!("skipping browser-dependent CLI test: no local browser found");
        return;
    };

    let temp = tempdir().unwrap();
    let output = temp.path().join("invalid.png");

    cargo_bin_cmd!("md2image")
        .timeout(Duration::from_secs(30))
        .env("MD2IMAGE_BROWSER", &browser_path)
        .arg("tests/fixtures/invalid_math.md")
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    assert!(output.exists());
}

fn browser_path_for_test() -> Option<std::path::PathBuf> {
    resolve_browser_path(None).ok()
}
