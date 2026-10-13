//! Helpers for converting reference sections back to text for assertions.

use super::reader::{Block, Link, Member, Section, Span, Token, TypePart, TypeText};

pub fn section<'a>(sections: &'a [Section], name: &str) -> &'a Section {
    sections
        .iter()
        .find(|section| section.name == name)
        .unwrap_or_else(|| panic!("the reference has no section {name}"))
}

pub fn member<'a>(section: &'a Section, name: &str) -> &'a Member {
    section
        .members
        .iter()
        .find(|member| member.name == name)
        .unwrap_or_else(|| panic!("{} has no member {name}", section.name))
}

/// Returns all spans in block order, including list items and headings.
pub fn spans(blocks: &[Block]) -> Vec<&Span> {
    blocks
        .iter()
        .flat_map(|block| match block {
            Block::Paragraph { spans } | Block::Heading { spans, .. } => {
                spans.iter().collect::<Vec<_>>()
            }
            Block::List { items } => items.iter().flat_map(|item| &item.spans).collect(),
            Block::Code { .. } => Vec::new(),
            Block::Table { head, rows } => head
                .iter()
                .chain(rows.iter().flat_map(|row| &row.cells))
                .flat_map(|cell| &cell.spans)
                .collect(),
        })
        .collect()
}

/// Concatenates block text with one line per paragraph, list item, heading or code block.
pub fn text(blocks: &[Block]) -> String {
    blocks
        .iter()
        .flat_map(|block| match block {
            Block::Paragraph { spans } => vec![join(spans)],
            Block::List { items } => items
                .iter()
                .map(|item| format!("* {}", join(&item.spans)))
                .collect(),
            Block::Heading { level, spans } => {
                vec![format!(
                    "{} {}",
                    "#".repeat(usize::from(*level)),
                    join(spans)
                )]
            }
            Block::Code { tokens, .. } => vec![code(tokens)],
            Block::Table { head, rows } => std::iter::once(head)
                .chain(rows.iter().map(|row| &row.cells))
                .map(|cells| {
                    cells
                        .iter()
                        .map(|cell| join(&cell.spans))
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .collect(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn join(spans: &[Span]) -> String {
    spans.iter().map(|span| span.text.as_str()).collect()
}

/// Returns `(text, link)` for each linked span in block order.
pub fn links(blocks: &[Block]) -> Vec<(String, Link)> {
    spans(blocks)
        .into_iter()
        .filter_map(|span| Some((span.text.clone(), span.link.clone()?)))
        .collect()
}

/// Concatenates example token text into source code.
pub fn code(tokens: &[Token]) -> String {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

pub fn type_text(type_text: &TypeText) -> String {
    type_text
        .parts
        .iter()
        .map(|part| match part {
            TypePart::Text { text } => text.as_str(),
            TypePart::Name { name } => name.as_str(),
        })
        .collect()
}
