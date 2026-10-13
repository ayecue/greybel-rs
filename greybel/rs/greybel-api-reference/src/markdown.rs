use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::anchors::AnchorTable;
use crate::example::ExampleHighlighter;
use crate::protocol::{Block, Highlight, Link, ListItem, Span, Style, TableCell, TableRow, Token};

/// Converts Markdown to styled paragraph, list, heading, code, and table blocks. Code blocks with no fence language
/// or a [`SCRIPT_LANGUAGES`] fence are highlighted; others remain plain. Quotes become paragraphs, nested lists
/// flatten into the outer list, and list-item headings, code blocks, and tables remain text. HTML is plain text.
pub(crate) struct MarkdownReader<'a> {
    anchors: &'a AnchorTable,
    highlighter: &'a ExampleHighlighter,
    blocks: Vec<Block>,
    items: Option<Vec<ListItem>>,
    /// Current list depth. Nested items append to the outermost list.
    depth: u32,
    spans: Vec<Span>,
    heading: Option<u8>,
    code: Option<Code>,
    table: Option<Table>,
    emphasis: u32,
    strong: u32,
    link: Option<Link>,
}

impl<'a> MarkdownReader<'a> {
    pub(crate) fn new(anchors: &'a AnchorTable, highlighter: &'a ExampleHighlighter) -> Self {
        Self {
            anchors,
            highlighter,
            blocks: Vec::new(),
            items: None,
            depth: 0,
            spans: Vec::new(),
            heading: None,
            code: None,
            table: None,
            emphasis: 0,
            strong: 0,
            link: None,
        }
    }

    pub(crate) fn read(mut self, markdown: &str) -> Vec<Block> {
        for event in Parser::new_ext(markdown, Options::ENABLE_TABLES) {
            self.take(event);
        }

        self.end_paragraph();

        self.blocks
    }

    fn take(&mut self, event: Event<'_>) {
        match event {
            Event::Start(Tag::List(_)) => {
                if self.depth == 0 {
                    self.end_paragraph();
                    self.items = Some(Vec::new());
                } else {
                    self.end_item();
                }

                self.depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                self.depth = self.depth.saturating_sub(1);

                if self.depth == 0 {
                    self.end_list();
                } else {
                    self.end_item();
                }
            }
            Event::Start(Tag::Item) | Event::End(TagEnd::Item) => self.end_item(),
            Event::Start(Tag::Heading { level, .. }) if self.items.is_none() => {
                self.end_paragraph();
                self.heading = Some(level as u8);
            }
            Event::End(TagEnd::Heading(_)) if self.items.is_none() => self.end_heading(),
            Event::Start(Tag::Table(_)) if self.items.is_none() => {
                self.end_paragraph();
                self.table = Some(Table::default());
            }
            Event::Start(Tag::TableCell) => self.spans.clear(),
            Event::End(TagEnd::TableCell) => {
                if let Some(table) = &mut self.table {
                    table.cells.push(TableCell {
                        spans: std::mem::take(&mut self.spans),
                    });
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = &mut self.table {
                    table.head = std::mem::take(&mut table.cells);
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(table) = &mut self.table {
                    let cells = std::mem::take(&mut table.cells);

                    table.rows.push(TableRow { cells });
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(Table { head, rows, .. }) = self.table.take() {
                    self.blocks.push(Block::Table { head, rows });
                }
            }
            Event::Start(Tag::CodeBlock(kind)) if self.items.is_none() => {
                self.end_paragraph();
                self.code = Some(Code::new(&kind));
            }
            Event::End(TagEnd::CodeBlock) if self.items.is_none() => self.end_code(),
            Event::Text(text) if self.code.is_some() => {
                if let Some(code) = &mut self.code {
                    code.text.push_str(&text);
                }
            }
            Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::BlockQuote(_) | TagEnd::CodeBlock,
            ) => {
                if self.items.is_none() {
                    self.end_paragraph();
                }
            }
            Event::Start(Tag::Emphasis) => self.emphasis += 1,
            Event::End(TagEnd::Emphasis) => self.emphasis = self.emphasis.saturating_sub(1),
            Event::Start(Tag::Strong) => self.strong += 1,
            Event::End(TagEnd::Strong) => self.strong = self.strong.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => self.link = self.anchors.resolve(&dest_url),
            Event::End(TagEnd::Link) => self.link = None,
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.push(&text, self.style());
            }
            Event::Code(code) => self.push(&code, Style::Code),
            Event::SoftBreak => self.push(" ", self.style()),
            Event::HardBreak => self.push("\n", self.style()),
            _ => {}
        }
    }

    fn style(&self) -> Style {
        if self.strong > 0 {
            Style::Strong
        } else if self.emphasis > 0 {
            Style::Emphasis
        } else {
            Style::Plain
        }
    }

    /// Appends styled `text`, merging with the previous span when style and link are equal.
    fn push(&mut self, text: &str, style: Style) {
        if let Some(last) = self.spans.last_mut()
            && last.style == style
            && last.link == self.link
        {
            last.text.push_str(text);

            return;
        }

        self.spans.push(Span {
            text: text.to_owned(),
            style,
            link: self.link.clone(),
        });
    }

    fn end_paragraph(&mut self) {
        if !self.spans.is_empty() {
            let spans = std::mem::take(&mut self.spans);

            self.blocks.push(Block::Paragraph { spans });
        }
    }

    fn end_heading(&mut self) {
        if let Some(level) = self.heading.take() {
            let spans = std::mem::take(&mut self.spans);

            self.blocks.push(Block::Heading { level, spans });
        }
    }

    fn end_code(&mut self) {
        if let Some(Code { language, text }) = self.code.take() {
            let text = text.trim_end_matches('\n');
            let tokens = match &language {
                None => self.highlighter.code(text),
                Some(_) => vec![Token {
                    text: text.to_owned(),
                    highlight: Highlight::Plain,
                    link: None,
                }],
            };

            self.blocks.push(Block::Code { tokens, language });
        }
    }

    fn end_item(&mut self) {
        if let Some(items) = &mut self.items
            && !self.spans.is_empty()
        {
            items.push(ListItem {
                spans: std::mem::take(&mut self.spans),
            });
        }
    }

    fn end_list(&mut self) {
        self.end_item();
        if let Some(items) = self.items.take()
            && !items.is_empty()
        {
            self.blocks.push(Block::List { items });
        }
    }
}

/// Code-fence languages recognized as GreyScript.
pub(crate) const SCRIPT_LANGUAGES: [&str; 2] = ["greyscript", "greybel"];

/// In-progress table state: header cells, body rows, and current-row cells.
#[derive(Default)]
struct Table {
    head: Vec<TableCell>,
    rows: Vec<TableRow>,
    cells: Vec<TableCell>,
}

/// In-progress code block: non-GreyScript fence language and source text.
struct Code {
    language: Option<String>,
    text: String,
}

impl Code {
    fn new(kind: &CodeBlockKind<'_>) -> Self {
        let language = match kind {
            CodeBlockKind::Fenced(info) => info.split_whitespace().next(),
            CodeBlockKind::Indented => None,
        };

        Self {
            language: language
                .filter(|language| !SCRIPT_LANGUAGES.contains(language))
                .map(str::to_owned),
            text: String::new(),
        }
    }
}
