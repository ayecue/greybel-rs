//! Serializes reference sections as Markdown for text-only consumers such as models.

use std::fmt::Write as _;

use crate::protocol::{
    Argument, Block, Link, Literal, Member, MemberKind, Section, Span, Style, TableCell, Token,
    TypePart, TypeText,
};

impl Section {
    /// Serializes a section's name, base type, description, example, and members as Markdown, fencing examples with
    /// `language`.
    #[must_use]
    pub fn markdown(&self, language: &str) -> String {
        let mut text = format!("# {}\n", self.name);

        if let Some(extends) = &self.extends {
            let _ = writeln!(text, "\nExtends `{extends}`.");
        }

        write_description(&mut text, &self.description, &self.example, language);

        for member in &self.members {
            text.push('\n');
            text.push_str(&member.markdown(&self.name, language));
        }

        text
    }
}

impl Member {
    /// Formats a member function or value signature, for example
    /// `replace(pattern: string, newValue: string = ""): string`.
    #[must_use]
    pub fn signature(&self) -> String {
        match &self.kind {
            MemberKind::Function {
                arguments, returns, ..
            } => {
                let arguments = arguments
                    .iter()
                    .map(Argument::text)
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{}({arguments}): {}", self.name, types_text(returns))
            }
            MemberKind::Value { r#type } => format!("{}: {}", self.name, r#type.to_text()),
        }
    }

    /// Serializes `owner`'s member as Markdown with its signature, known return values, tags, description, and example,
    /// using `language` for the example fence.
    #[must_use]
    pub fn markdown(&self, owner: &str, language: &str) -> String {
        let mut text = format!("## `{owner}.{}`\n", self.signature());

        if let MemberKind::Function {
            return_variations, ..
        } = &self.kind
            && !return_variations.is_empty()
        {
            let variations = return_variations
                .iter()
                .map(|literal| format!("`{}`", literal.to_text()))
                .collect::<Vec<_>>()
                .join(", ");

            let _ = writeln!(text, "\nKnown return values: {variations}.");
        }

        if !self.tags.is_empty() {
            let _ = writeln!(text, "\nTags: {}.", self.tags.join(", "));
        }

        write_description(&mut text, &self.description, &self.example, language);

        text
    }
}

impl Argument {
    fn text(&self) -> String {
        let mut text = self.name.clone();

        if self.optional && self.default.is_none() {
            text.push('?');
        }

        if !self.types.is_empty() {
            let _ = write!(text, ": {}", types_text(&self.types));
        }

        if let Some(default) = &self.default {
            let _ = write!(text, " = {}", default.to_text());
        }

        text
    }
}

impl TypeText {
    /// Formats type syntax such as `list<map<string,number>>`.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.parts
            .iter()
            .map(|part| match part {
                TypePart::Text { text } => text.as_str(),
                TypePart::Name { name } => name.as_str(),
            })
            .collect()
    }
}

impl Literal {
    /// Formats a GreyScript literal, doubling quotes inside strings.
    #[must_use]
    pub fn to_text(&self) -> String {
        match self {
            Self::String { value } => format!("\"{}\"", value.replace('"', "\"\"")),
            Self::Number { value } => value.to_string(),
        }
    }
}

/// Joins union alternatives as `a | b`; an empty type list becomes `null`.
fn types_text(types: &[TypeText]) -> String {
    if types.is_empty() {
        return "null".to_owned();
    }

    types
        .iter()
        .map(TypeText::to_text)
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Appends description paragraphs and list items, then an optional `language`-fenced example.
fn write_description(text: &mut String, description: &[Block], example: &[Token], language: &str) {
    for block in description {
        text.push('\n');

        match block {
            Block::Paragraph { spans } => {
                text.push_str(&spans_text(spans));
                text.push('\n');
            }
            Block::List { items } => {
                for item in items {
                    let _ = writeln!(text, "- {}", spans_text(&item.spans));
                }
            }
            Block::Heading { level, spans } => {
                let marks = "#".repeat(usize::from(*level));

                let _ = writeln!(text, "{marks} {}", spans_text(spans));
            }
            Block::Table { head, rows } => {
                let _ = writeln!(text, "{}", row_text(head));
                let _ = writeln!(text, "|{}", " --- |".repeat(head.len()));

                for row in rows {
                    let _ = writeln!(text, "{}", row_text(&row.cells));
                }
            }
            Block::Code {
                tokens,
                language: code_language,
            } => {
                let fence = code_language.as_deref().unwrap_or(language);

                let _ = writeln!(text, "```{fence}\n{}\n```", code_text(tokens));
            }
        }
    }

    if !example.is_empty() {
        let _ = writeln!(text, "\n```{language}\n{}\n```", code_text(example));
    }
}

/// Builds a Markdown table row from `cells`, escaping pipes.
fn row_text(cells: &[TableCell]) -> String {
    let cells: String = cells
        .iter()
        .map(|cell| format!(" {} |", spans_text(&cell.spans).replace('|', "\\|")))
        .collect();

    format!("|{cells}")
}

fn code_text(tokens: &[Token]) -> String {
    let code: String = tokens.iter().map(|token| token.text.as_str()).collect();

    code.trim_end().to_owned()
}

fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(span_text).collect()
}

fn span_text(span: &Span) -> String {
    let text = match span.style {
        Style::Plain => span.text.clone(),
        Style::Emphasis => format!("*{}*", span.text),
        Style::Strong => format!("**{}**", span.text),
        Style::Code => format!("`{}`", span.text),
    };

    match &span.link {
        Some(Link::Web { url }) => format!("[{text}]({url})"),
        Some(Link::Type { .. } | Link::Member { .. } | Link::Page { .. }) | None => text,
    }
}
