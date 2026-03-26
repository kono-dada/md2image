use crate::error::{AppError, Result};

const BODY_MAX_WIDTH: u32 = 980;

const GITHUB_LIGHT_CSS: &str = include_str!("../vendor/themes/github-markdown-light.css");
const PICO_CSS: &str = include_str!("../vendor/themes/pico.classless.min.css");
const SPLENDOR_CSS: &str = include_str!("../vendor/themes/splendor.css");

struct Theme {
    article_class: &'static str,
    body_background: &'static str,
    body_color: &'static str,
    article_background: &'static str,
    article_border: &'static str,
    article_shadow: &'static str,
    article_padding: &'static str,
    stylesheet: &'static str,
    extra_css: &'static str,
}

pub fn build_html(markdown_html: &str, width: u32, theme_name: &str) -> Result<String> {
    let theme = theme(theme_name).ok_or_else(|| AppError::UnsupportedTheme {
        theme: theme_name.to_string(),
    })?;
    let body_width = width.saturating_sub(80).clamp(320, BODY_MAX_WIDTH);
    let article_class_attr = if theme.article_class.is_empty() {
        "md2image-page".to_string()
    } else {
        format!("md2image-page {}", theme.article_class)
    };

    Ok(format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>md2image</title>
    <style>
{stylesheet}

      html, body {{
        margin: 0;
        padding: 0;
        width: 100%;
      }}

      * {{
        box-sizing: border-box;
      }}

      body {{
        padding: 40px 24px;
        background: {body_background};
        color: {body_color};
      }}

      .md2image-page {{
        width: min(100%, {body_width}px);
        margin: 0 auto;
        padding: {article_padding};
        background: {article_background};
        border: 1px solid {article_border};
        box-shadow: {article_shadow};
        border-radius: 20px;
        overflow-wrap: break-word;
      }}

      .md2image-page > :first-child {{
        margin-top: 0;
      }}

      .md2image-page img {{
        max-width: 100%;
        height: auto;
      }}

{extra_css}
    </style>
  </head>
  <body>
    <article class="{article_class_attr}">
      {markdown_html}
    </article>
  </body>
</html>
"#,
        stylesheet = theme.stylesheet,
        body_background = theme.body_background,
        body_color = theme.body_color,
        body_width = body_width,
        article_padding = theme.article_padding,
        article_background = theme.article_background,
        article_border = theme.article_border,
        article_shadow = theme.article_shadow,
        extra_css = theme.extra_css,
        article_class_attr = article_class_attr,
        markdown_html = markdown_html,
    ))
}

pub fn supported_themes() -> &'static [&'static str] {
    &["default", "github-light", "pico", "splendor"]
}

fn theme(name: &str) -> Option<Theme> {
    match name {
        "default" | "github-light" => Some(Theme {
            article_class: "markdown-body",
            body_background: "#f6f8fa",
            body_color: "#1f2328",
            article_background: "#ffffff",
            article_border: "#d0d7de",
            article_shadow: "0 24px 70px rgba(31, 35, 40, 0.08)",
            article_padding: "45px",
            stylesheet: GITHUB_LIGHT_CSS,
            extra_css: r#"
      .md2image-page.markdown-body {
        min-width: 200px;
      }
    "#,
        }),
        "pico" => Some(Theme {
            article_class: "",
            body_background: "#f3f5f7",
            body_color: "#1e293b",
            article_background: "#ffffff",
            article_border: "#dfe7ef",
            article_shadow: "0 24px 70px rgba(15, 23, 42, 0.10)",
            article_padding: "42px 46px",
            stylesheet: PICO_CSS,
            extra_css: r#"
      .md2image-page {
        --pico-font-family: "Inter", "Segoe UI", -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif;
        --pico-border-radius: 16px;
      }

      .md2image-page h1,
      .md2image-page h2,
      .md2image-page h3 {
        letter-spacing: -0.03em;
      }
    "#,
        }),
        "splendor" => Some(Theme {
            article_class: "",
            body_background: "#f3eee7",
            body_color: "#2b241e",
            article_background: "#fffdf9",
            article_border: "#eadfce",
            article_shadow: "0 26px 70px rgba(73, 53, 35, 0.12)",
            article_padding: "52px 58px",
            stylesheet: SPLENDOR_CSS,
            extra_css: r#"
      .md2image-page {
        border-radius: 28px;
      }
    "#,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_html, supported_themes};

    #[test]
    fn supports_expected_theme_names() {
        assert_eq!(
            supported_themes(),
            &["default", "github-light", "pico", "splendor"]
        );
    }

    #[test]
    fn embeds_selected_theme_stylesheet() {
        let html = build_html("<h1>Hello</h1>", 960, "pico").expect("theme should build");
        assert!(html.contains("--pico-font-size"));
    }
}
