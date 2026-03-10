use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

use crate::browser::resolve_browser_path;
use crate::cli::Cli;
use crate::error::{AppError, Result};
use crate::html::build_html;
use crate::input::read_markdown;
use crate::markdown::parse;
use crate::render::{ChromiumRenderer, RenderOptions, Renderer};

pub fn run(cli: Cli) -> Result<()> {
    let started_at = Instant::now();
    let mut timings = Vec::new();

    validate_cli(&cli)?;

    let step_started = Instant::now();
    let markdown = read_markdown(cli.input.as_deref())?;
    record_timing(&mut timings, "read input", step_started);

    let step_started = Instant::now();
    let browser_path = resolve_browser_path(cli.browser.as_deref())?;
    record_timing(&mut timings, "resolve browser", step_started);

    let step_started = Instant::now();
    let document = parse(&markdown);
    record_timing(&mut timings, "parse markdown", step_started);

    let step_started = Instant::now();
    let html = build_html(&document, cli.width, &cli.theme);
    record_timing(&mut timings, "build html", step_started);

    let renderer = ChromiumRenderer;
    let step_started = Instant::now();
    let png = renderer.render(
        &html,
        &RenderOptions {
            width: cli.width,
            scale: cli.scale,
            supersample: cli.supersample,
            timing: cli.timing,
            browser_path,
        },
    )?;
    record_timing(&mut timings, "render png", step_started);

    let step_started = Instant::now();
    write_output(&cli.output, &png)
        .inspect(|_| record_timing(&mut timings, "write output", step_started))?;

    if cli.timing {
        print_timings(&timings, started_at.elapsed());
    }

    Ok(())
}

fn validate_cli(cli: &Cli) -> Result<()> {
    if cli.theme != "default" {
        return Err(AppError::UnsupportedTheme {
            theme: cli.theme.clone(),
        });
    }

    if cli.width == 0 {
        return Err(AppError::Usage {
            message: "`--width` must be greater than 0.".to_string(),
        });
    }

    if !cli.scale.is_finite() || cli.scale < 1.0 {
        return Err(AppError::Usage {
            message: "`--scale` must be a finite number greater than or equal to 1.".to_string(),
        });
    }

    if !cli.supersample.is_finite() || cli.supersample < 1.0 {
        return Err(AppError::Usage {
            message: "`--supersample` must be a finite number greater than or equal to 1."
                .to_string(),
        });
    }

    Ok(())
}

fn write_output(path: &Path, png: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    fs::create_dir_all(parent).map_err(|source| AppError::CreateOutputDir {
        path: parent.to_path_buf(),
        source,
    })?;

    let mut temp = NamedTempFile::new_in(parent).map_err(|source| AppError::WriteOutput {
        path: path.to_path_buf(),
        source,
    })?;

    temp.write_all(png)
        .map_err(|source| AppError::WriteOutput {
            path: path.to_path_buf(),
            source,
        })?;

    temp.flush().map_err(|source| AppError::WriteOutput {
        path: path.to_path_buf(),
        source,
    })?;

    match temp.persist(path) {
        Ok(_) => Ok(()),
        Err(error) => {
            let source = error.error;
            if path.exists() {
                fs::remove_file(path).map_err(|remove_error| AppError::WriteOutput {
                    path: path.to_path_buf(),
                    source: remove_error,
                })?;

                fs::rename(error.file.path(), path).map_err(|rename_error| {
                    AppError::WriteOutput {
                        path: path.to_path_buf(),
                        source: rename_error,
                    }
                })?;

                Ok(())
            } else {
                Err(AppError::WriteOutput {
                    path: path.to_path_buf(),
                    source,
                })
            }
        }
    }
}

fn record_timing(
    timings: &mut Vec<(&'static str, Duration)>,
    label: &'static str,
    started_at: Instant,
) {
    timings.push((label, started_at.elapsed()));
}

fn print_timings(timings: &[(&'static str, Duration)], total: Duration) {
    eprintln!("md2image timing:");
    for (label, duration) in timings {
        eprintln!("  {label:>16}: {}", format_duration(*duration));
    }
    eprintln!("  {:>16}: {}", "total", format_duration(total));
}

fn format_duration(duration: Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
}
