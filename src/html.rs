use html_escape::encode_text;

use crate::katex;
use crate::markdown::{Block, Document, Inline, ListBlock};

const BODY_MAX_WIDTH: u32 = 760;
const MATH_BOOTSTRAP_SCRIPT: &str = r#"
      window.__md2imageMathStatus = { done: false, ok: true, error: "" };
      (() => {
        const status = window.__md2imageMathStatus;
        const summarize = value => value.replace(/\s+/g, " ").trim().slice(0, 120);

        try {
          for (const element of document.querySelectorAll("[data-md2image-math]")) {
            const expression = element.textContent || "";
            const displayMode = element.getAttribute("data-md2image-math") === "display";
            element.textContent = "";
            katex.render(expression, element, { displayMode, throwOnError: false });
            element.setAttribute("data-md2image-math-rendered", "true");
          }

          status.done = true;
        } catch (error) {
          const message = error && error.message ? error.message : String(error);
          const failedNode = document.querySelector("[data-md2image-math]:not([data-md2image-math-rendered])");
          const expression = failedNode ? summarize(failedNode.textContent || "") : "";
          status.ok = false;
          status.done = true;
          status.error = expression ? `${expression}: ${message}` : message;
        }
      })();
"#;

pub fn build_html(document: &Document, width: u32, theme: &str) -> String {
    let content = render_blocks(&document.blocks);
    let body_width = width.saturating_sub(80).clamp(320, BODY_MAX_WIDTH);

    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>md2image</title>
    <link rel="stylesheet" href="{katex_stylesheet_path}">
    <script src="{katex_script_path}"></script>
    <style>
      :root {{
        color-scheme: light;
        --page-width: {body_width}px;
        --bg: #f4f1ea;
        --paper: #fffdf8;
        --text: #1e1b18;
        --muted: #6b6258;
        --border: #ded5c8;
        --quote: #c06c45;
        --code-bg: #f3ede2;
        --rule: #d9cfbf;
        --link: #8a3b12;
      }}

      * {{
        box-sizing: border-box;
      }}

      html, body {{
        margin: 0;
        padding: 0;
        width: 100%;
        background:
          radial-gradient(circle at top left, rgba(192, 108, 69, 0.08), transparent 32%),
          linear-gradient(180deg, #f8f4ec 0%, #efe7da 100%);
        color: var(--text);
        font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Palatino, Georgia, serif;
      }}

      body {{
        padding: 40px;
      }}

      .page {{
        width: min(100%, var(--page-width));
        margin: 0 auto;
        padding: 44px 48px;
        background: var(--paper);
        border: 1px solid rgba(222, 213, 200, 0.9);
        border-radius: 20px;
        box-shadow: 0 18px 60px rgba(82, 63, 47, 0.13);
      }}

      h1, h2, h3, h4, h5, h6 {{
        margin: 0 0 0.8em;
        line-height: 1.15;
        letter-spacing: -0.02em;
      }}

      h1 {{ font-size: 2.3rem; }}
      h2 {{ font-size: 1.9rem; }}
      h3 {{ font-size: 1.55rem; }}
      h4 {{ font-size: 1.3rem; }}
      h5 {{ font-size: 1.1rem; }}
      h6 {{ font-size: 1rem; }}

      p, li, blockquote, pre {{
        font-size: 1.05rem;
        line-height: 1.72;
      }}

      p, ul, ol, blockquote, pre, hr {{
        margin: 0 0 1.1rem;
      }}

      ul, ol {{
        padding-left: 1.5rem;
      }}

      li + li {{
        margin-top: 0.25rem;
      }}

      blockquote {{
        margin-left: 0;
        padding: 0.1rem 0 0.1rem 1rem;
        color: var(--muted);
        border-left: 4px solid var(--quote);
      }}

      code {{
        font-family: "SFMono-Regular", "SF Mono", Menlo, Monaco, Consolas, "Liberation Mono", monospace;
        font-size: 0.92em;
      }}

      p code, li code, blockquote code {{
        padding: 0.14rem 0.35rem;
        border-radius: 0.35rem;
        background: var(--code-bg);
      }}

      pre {{
        padding: 1rem 1.1rem;
        overflow-x: auto;
        background: #1f1d1a;
        color: #f8f3ea;
        border-radius: 14px;
      }}

      pre code {{
        font-size: 0.92rem;
        white-space: pre-wrap;
        word-break: break-word;
      }}

      hr {{
        border: none;
        border-top: 1px solid var(--rule);
      }}

      a {{
        color: var(--link);
        text-decoration: none;
        border-bottom: 1px solid rgba(138, 59, 18, 0.25);
      }}

      strong {{
        font-weight: 700;
      }}

      em {{
        font-style: italic;
      }}

      .md2image-math-inline .katex {{
        font-size: 1em;
      }}

      .md2image-math-display {{
        display: block;
        margin: 0 0 1.1rem;
        overflow-x: auto;
        overflow-y: hidden;
        padding: 0.2rem 0;
      }}

      .md2image-math-display .katex-display {{
        margin: 0;
      }}
    </style>
  </head>
  <body data-theme="{theme}">
    <main class="page">
      {content}
    </main>
    <script>
{math_bootstrap_script}
    </script>
  </body>
</html>
"#,
        katex_stylesheet_path = katex::STYLESHEET_PATH,
        katex_script_path = katex::SCRIPT_PATH,
        math_bootstrap_script = MATH_BOOTSTRAP_SCRIPT,
    )
}

fn render_blocks(blocks: &[Block]) -> String {
    let mut html = String::new();

    for block in blocks {
        match block {
            Block::Heading { level, content } => {
                html.push_str(&format!("<h{level}>{}</h{level}>", render_inlines(content)));
            }
            Block::Paragraph(content) => {
                html.push_str(&format!("<p>{}</p>", render_inlines(content)));
            }
            Block::DisplayMath(expression) => html.push_str(&render_display_math(expression)),
            Block::BlockQuote(children) => {
                html.push_str("<blockquote>");
                html.push_str(&render_blocks(children));
                html.push_str("</blockquote>");
            }
            Block::List(list) => html.push_str(&render_list(list)),
            Block::CodeBlock { language, code } => {
                let language_attr = language
                    .as_deref()
                    .map(|lang| format!(" data-language=\"{}\"", encode_text(lang)))
                    .unwrap_or_default();
                html.push_str(&format!(
                    "<pre><code{language_attr}>{}</code></pre>",
                    encode_text(code)
                ));
            }
            Block::ThematicBreak => html.push_str("<hr>"),
        }
    }

    html
}

fn render_list(list: &ListBlock) -> String {
    let mut html = String::new();
    let tag = if list.ordered { "ol" } else { "ul" };

    if list.ordered {
        match list.start {
            Some(start) if start > 1 => html.push_str(&format!("<ol start=\"{start}\">")),
            _ => html.push_str("<ol>"),
        }
    } else {
        html.push_str("<ul>");
    }

    for item in &list.items {
        html.push_str("<li>");
        html.push_str(&render_blocks(item));
        html.push_str("</li>");
    }

    html.push_str(&format!("</{tag}>"));
    html
}

fn render_inlines(inlines: &[Inline]) -> String {
    let mut html = String::new();

    for inline in inlines {
        match inline {
            Inline::Text(text) => html.push_str(&encode_text(text)),
            Inline::Math(expression) => html.push_str(&format!(
                "<span class=\"md2image-math-inline\" data-md2image-math=\"inline\">{}</span>",
                encode_text(expression)
            )),
            Inline::DisplayMath(expression) => html.push_str(&format!(
                "<span class=\"md2image-math-inline\" data-md2image-math=\"display\">{}</span>",
                encode_text(expression.trim())
            )),
            Inline::Strong(children) => {
                html.push_str("<strong>");
                html.push_str(&render_inlines(children));
                html.push_str("</strong>");
            }
            Inline::Emphasis(children) => {
                html.push_str("<em>");
                html.push_str(&render_inlines(children));
                html.push_str("</em>");
            }
            Inline::Code(code) => {
                html.push_str("<code>");
                html.push_str(&encode_text(code));
                html.push_str("</code>");
            }
            Inline::Link { text, destination } => {
                html.push_str(&format!(
                    "<a href=\"{}\">{}</a>",
                    encode_text(destination),
                    render_inlines(text)
                ));
            }
            Inline::SoftBreak => html.push('\n'),
            Inline::HardBreak => html.push_str("<br>"),
        }
    }

    html
}

fn render_display_math(expression: &str) -> String {
    format!(
        "<div class=\"md2image-math-display\" data-md2image-math=\"display\">{}</div>",
        encode_text(expression)
    )
}

#[cfg(test)]
mod tests {
    use crate::markdown::{Block, Document, Inline, ListBlock, parse};

    #[test]
    fn builds_stable_html() {
        let document = Document {
            blocks: vec![
                Block::Heading {
                    level: 1,
                    content: vec![Inline::Text("Title".into())],
                },
                Block::Paragraph(vec![
                    Inline::Text("Text with ".into()),
                    Inline::Strong(vec![Inline::Text("bold".into())]),
                ]),
                Block::DisplayMath("E = mc^2".into()),
                Block::List(ListBlock {
                    ordered: false,
                    start: None,
                    items: vec![vec![Block::Paragraph(vec![Inline::Text("Item".into())])]],
                }),
            ],
        };

        let html = super::build_html(&document, 960, "default");
        assert!(html.contains("class=\"page\""));
        assert!(html.contains("katex.min.css"));
        assert!(html.contains("window.__md2imageMathStatus"));
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("data-md2image-math=\"display\""));
        assert!(html.contains("<ul>"));
    }

    #[test]
    fn keeps_tight_list_item_inline_code_in_one_html_paragraph() {
        let document = parse(
            "- `--scale <MULTIPLIER>`：可选，默认 `1.0`，例如 `--width 960 --scale 2` 会输出约 `1920px` 宽的 PNG。",
        );

        let html = super::build_html(&document, 960, "default");
        assert!(html.contains(
            "<li><p><code>--scale &lt;MULTIPLIER&gt;</code>：可选，默认 <code>1.0</code>"
        ));
        assert!(html.contains(
            "<code>--width 960 --scale 2</code> 会输出约 <code>1920px</code> 宽的 PNG。</p></li>"
        ));
        assert!(!html.contains("</p><p>"));
    }

    #[test]
    fn renders_inline_math_and_bootstrap_script() {
        let document = parse("Inline $x^2$ formula.");

        let html = super::build_html(&document, 960, "default");
        assert!(html.contains("window.__md2imageMathStatus"));
        assert!(html.contains("data-md2image-math=\"inline\">x^2</span>"));
    }

    #[test]
    fn renders_display_math_outside_paragraph_tags() {
        let document = parse("$$\nE = mc^2\n$$\n");

        let html = super::build_html(&document, 960, "default");
        assert!(html.contains(
            "<div class=\"md2image-math-display\" data-md2image-math=\"display\">E = mc^2</div>"
        ));
        assert!(!html.contains("<p><div"));
    }
}
