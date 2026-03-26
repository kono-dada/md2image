use pulldown_cmark::{Options, Parser, html};

pub fn render_html(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, markdown_options());
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options
}

#[cfg(test)]
mod tests {
    use super::render_html;

    #[test]
    fn renders_commonmark_image_and_raw_html() {
        let html = render_html("before ![alt](image.png) after\n\n<div>safe enough</div>");

        assert!(html.contains(r#"<img src="image.png" alt="alt" />"#));
        assert!(html.contains("<div>safe enough</div>"));
    }

    #[test]
    fn renders_tables_and_task_lists() {
        let html = render_html("| A | B |\n| - | - |\n| 1 | 2 |\n\n- [x] done");

        assert!(html.contains("<table>"));
        assert!(html.contains(r#"type="checkbox""#));
    }
}
