use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    BlockQuote(Vec<Block>),
    List(ListBlock),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBlock {
    pub ordered: bool,
    pub start: Option<u64>,
    pub items: Vec<Vec<Block>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Link {
        text: Vec<Inline>,
        destination: String,
    },
    SoftBreak,
    HardBreak,
}

pub fn parse(markdown: &str) -> Document {
    let events: Vec<Event<'_>> = Parser::new(markdown).collect();
    let mut parser = EventParser::new(events);
    parser.parse_document()
}

struct EventParser<'a> {
    events: Vec<Event<'a>>,
    cursor: usize,
}

impl<'a> EventParser<'a> {
    fn new(events: Vec<Event<'a>>) -> Self {
        Self { events, cursor: 0 }
    }

    fn parse_document(&mut self) -> Document {
        Document {
            blocks: self.parse_blocks_until(|_| false),
        }
    }

    fn parse_blocks_until(&mut self, is_end: impl Fn(&TagEnd) -> bool + Copy) -> Vec<Block> {
        let mut blocks = Vec::new();

        while let Some(event) = self.peek() {
            match event {
                Event::End(tag_end) if is_end(tag_end) => {
                    self.next();
                    break;
                }
                _ => {
                    if let Some(block) = self.parse_block() {
                        blocks.push(block);
                    } else {
                        self.next();
                    }
                }
            }
        }

        blocks
    }

    fn parse_block(&mut self) -> Option<Block> {
        match self.next()? {
            Event::Start(Tag::Paragraph) => {
                let content = self.parse_inlines_until(TagEnd::Paragraph);
                if content.is_empty() {
                    None
                } else {
                    Some(Block::Paragraph(normalize_inlines(content)))
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let content = self.parse_inlines_until(TagEnd::Heading(level));
                Some(Block::Heading {
                    level: heading_level(level),
                    content: normalize_inlines(content),
                })
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                Some(Block::BlockQuote(self.parse_blocks_until(
                    |end| matches!(end, TagEnd::BlockQuote(end_kind) if *end_kind == kind),
                )))
            }
            Event::Start(Tag::List(start)) => Some(Block::List(self.parse_list(start))),
            Event::Start(Tag::CodeBlock(kind)) => Some(self.parse_code_block(kind)),
            Event::Rule => Some(Block::ThematicBreak),
            Event::Html(text) => Some(Block::Paragraph(vec![Inline::Text(text.into_string())])),
            Event::InlineHtml(text) => {
                Some(Block::Paragraph(vec![Inline::Text(text.into_string())]))
            }
            Event::Text(text) => Some(Block::Paragraph(vec![Inline::Text(text.into_string())])),
            Event::Code(code) => Some(Block::Paragraph(vec![Inline::Code(code.into_string())])),
            Event::SoftBreak => Some(Block::Paragraph(vec![Inline::SoftBreak])),
            Event::HardBreak => Some(Block::Paragraph(vec![Inline::HardBreak])),
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                Some(Block::Paragraph(vec![Inline::Text(text.into_string())]))
            }
            Event::FootnoteReference(text) => {
                Some(Block::Paragraph(vec![Inline::Text(text.into_string())]))
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "[x]" } else { "[ ]" };
                Some(Block::Paragraph(vec![Inline::Text(marker.to_string())]))
            }
            Event::End(_) => None,
            Event::Start(Tag::HtmlBlock) => {
                let html = self.collect_raw_text_until(TagEnd::HtmlBlock);
                if html.is_empty() {
                    None
                } else {
                    Some(Block::Paragraph(vec![Inline::Text(html)]))
                }
            }
            Event::Start(Tag::FootnoteDefinition(_))
            | Event::Start(Tag::DefinitionList)
            | Event::Start(Tag::DefinitionListTitle)
            | Event::Start(Tag::DefinitionListDefinition)
            | Event::Start(Tag::Table(_))
            | Event::Start(Tag::TableHead)
            | Event::Start(Tag::TableRow)
            | Event::Start(Tag::TableCell)
            | Event::Start(Tag::Strikethrough)
            | Event::Start(Tag::Superscript)
            | Event::Start(Tag::Subscript)
            | Event::Start(Tag::MetadataBlock(_))
            | Event::Start(Tag::Image { .. })
            | Event::Start(Tag::Emphasis)
            | Event::Start(Tag::Strong)
            | Event::Start(Tag::Link { .. })
            | Event::Start(Tag::Item) => None,
        }
    }

    fn parse_list(&mut self, start: Option<u64>) -> ListBlock {
        let mut items = Vec::new();

        while let Some(event) = self.peek() {
            match event {
                Event::End(TagEnd::List(ordered)) => {
                    let ordered = *ordered;
                    self.next();
                    return ListBlock {
                        ordered,
                        start,
                        items,
                    };
                }
                Event::Start(Tag::Item) => {
                    self.next();
                    let item_blocks = self.parse_blocks_until(|end| matches!(end, TagEnd::Item));
                    items.push(item_blocks);
                }
                _ => {
                    self.next();
                }
            }
        }

        ListBlock {
            ordered: start.is_some(),
            start,
            items,
        }
    }

    fn parse_code_block(&mut self, kind: CodeBlockKind<'a>) -> Block {
        let language = match kind {
            CodeBlockKind::Indented => None,
            CodeBlockKind::Fenced(info) => {
                let language = info.split_whitespace().next().unwrap_or_default().trim();
                if language.is_empty() {
                    None
                } else {
                    Some(language.to_string())
                }
            }
        };

        let code = self.collect_raw_text_until(TagEnd::CodeBlock);

        Block::CodeBlock { language, code }
    }

    fn parse_inlines_until(&mut self, end_tag: TagEnd) -> Vec<Inline> {
        let mut inlines = Vec::new();

        while let Some(event) = self.next() {
            match event {
                Event::End(tag) if tag == end_tag => break,
                Event::Text(text) => inlines.push(Inline::Text(text.into_string())),
                Event::Code(code) => inlines.push(Inline::Code(code.into_string())),
                Event::SoftBreak => inlines.push(Inline::SoftBreak),
                Event::HardBreak => inlines.push(Inline::HardBreak),
                Event::InlineMath(text) | Event::DisplayMath(text) => {
                    inlines.push(Inline::Text(text.into_string()))
                }
                Event::Html(text) | Event::InlineHtml(text) => {
                    inlines.push(Inline::Text(text.into_string()))
                }
                Event::FootnoteReference(text) => inlines.push(Inline::Text(text.into_string())),
                Event::TaskListMarker(checked) => {
                    let marker = if checked { "[x]" } else { "[ ]" };
                    inlines.push(Inline::Text(marker.to_string()));
                }
                Event::Start(Tag::Emphasis) => inlines.push(Inline::Emphasis(normalize_inlines(
                    self.parse_inlines_until(TagEnd::Emphasis),
                ))),
                Event::Start(Tag::Strong) => inlines.push(Inline::Strong(normalize_inlines(
                    self.parse_inlines_until(TagEnd::Strong),
                ))),
                Event::Start(Tag::Link { dest_url, .. }) => {
                    let text = normalize_inlines(self.parse_inlines_until(TagEnd::Link));
                    inlines.push(Inline::Link {
                        text,
                        destination: dest_url.into_string(),
                    });
                }
                Event::Start(Tag::Image { .. }) => {
                    self.skip_until(TagEnd::Image);
                }
                Event::Start(Tag::HtmlBlock) => {
                    let html = self.collect_raw_text_until(TagEnd::HtmlBlock);
                    if !html.is_empty() {
                        inlines.push(Inline::Text(html));
                    }
                }
                Event::Start(Tag::Paragraph)
                | Event::Start(Tag::Heading { .. })
                | Event::Start(Tag::BlockQuote(_))
                | Event::Start(Tag::CodeBlock(_))
                | Event::Start(Tag::List(_))
                | Event::Start(Tag::Item)
                | Event::Start(Tag::FootnoteDefinition(_))
                | Event::Start(Tag::DefinitionList)
                | Event::Start(Tag::DefinitionListTitle)
                | Event::Start(Tag::DefinitionListDefinition)
                | Event::Start(Tag::Table(_))
                | Event::Start(Tag::TableHead)
                | Event::Start(Tag::TableRow)
                | Event::Start(Tag::TableCell)
                | Event::Start(Tag::Strikethrough)
                | Event::Start(Tag::Superscript)
                | Event::Start(Tag::Subscript)
                | Event::Start(Tag::MetadataBlock(_))
                | Event::Rule
                | Event::End(_) => {}
            }
        }

        inlines
    }

    fn collect_raw_text_until(&mut self, end_tag: TagEnd) -> String {
        let mut text = String::new();

        while let Some(event) = self.next() {
            match event {
                Event::End(tag) if tag == end_tag => break,
                Event::Text(value)
                | Event::Code(value)
                | Event::InlineMath(value)
                | Event::DisplayMath(value)
                | Event::Html(value)
                | Event::InlineHtml(value)
                | Event::FootnoteReference(value) => text.push_str(&value),
                Event::SoftBreak | Event::HardBreak => text.push('\n'),
                Event::TaskListMarker(checked) => {
                    text.push_str(if checked { "[x]" } else { "[ ]" });
                }
                Event::Rule => text.push_str("---"),
                Event::Start(tag) => self.skip_nested(tag.to_end()),
                Event::End(_) => {}
            }
        }

        text
    }

    fn skip_until(&mut self, end_tag: TagEnd) {
        while let Some(event) = self.next() {
            if matches!(event, Event::End(tag) if tag == end_tag) {
                break;
            }
        }
    }

    fn skip_nested(&mut self, end_tag: TagEnd) {
        let mut depth = 1usize;

        while let Some(event) = self.next() {
            match event {
                Event::Start(tag) if tag.to_end() == end_tag => depth += 1,
                Event::End(tag) if tag == end_tag => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn peek(&self) -> Option<&Event<'a>> {
        self.events.get(self.cursor)
    }

    fn next(&mut self) -> Option<Event<'a>> {
        let event = self.events.get(self.cursor)?.clone();
        self.cursor += 1;
        Some(event)
    }
}

fn normalize_inlines(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut normalized = Vec::new();

    for inline in inlines {
        match inline {
            Inline::Text(text) => {
                if text.is_empty() {
                    continue;
                }

                if let Some(Inline::Text(last)) = normalized.last_mut() {
                    last.push_str(&text);
                } else {
                    normalized.push(Inline::Text(text));
                }
            }
            other => normalized.push(other),
        }
    }

    normalized
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::{Block, Inline, parse};

    #[test]
    fn parses_basic_blocks_and_inlines() {
        let document = parse(
            "# Title\n\nParagraph with **bold** and *italic* and `code`.\n\n> Quote\n\n1. First\n2. Second\n\n---\n",
        );

        assert!(matches!(
            document.blocks[0],
            Block::Heading { level: 1, .. }
        ));
        assert!(matches!(document.blocks[1], Block::Paragraph(_)));
        assert!(matches!(document.blocks[2], Block::BlockQuote(_)));
        assert!(matches!(document.blocks[3], Block::List(_)));
        assert!(matches!(document.blocks[4], Block::ThematicBreak));
    }

    #[test]
    fn ignores_images_and_treats_html_as_text() {
        let document = parse("before ![alt](image.png) after\n\n<div>unsafe</div>");

        assert_eq!(
            document.blocks[0],
            Block::Paragraph(vec![Inline::Text("before  after".to_string())])
        );
        assert_eq!(
            document.blocks[1],
            Block::Paragraph(vec![Inline::Text("<div>unsafe</div>".to_string())])
        );
    }
}
