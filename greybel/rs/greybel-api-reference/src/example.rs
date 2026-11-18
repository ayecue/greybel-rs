use greybel_core::highlight::{HighlightKind, Highlighter};
use greybel_core::{Parser, ParserOptions};
use meta_utils::{BaseType, Signature};

use crate::api;
use crate::protocol::{Highlight, Link, Token};

/// Highlights GreyScript examples as the language server does and links general API function calls.
pub(crate) struct ExampleHighlighter {
    general: Option<&'static Signature>,
}

impl ExampleHighlighter {
    pub(crate) fn new() -> Self {
        Self {
            general: api::container().signature(BaseType::General.as_str()),
        }
    }

    /// Highlights tokens from `lines` joined by newlines.
    pub(crate) fn tokens(&self, lines: &[String]) -> Vec<Token> {
        self.code(&lines.join("\n"))
    }

    /// Highlights `code` with the AST of a recovering parse; the source after the first lexer error is plain text.
    pub(crate) fn code(&self, code: &str) -> Vec<Token> {
        let ast = Parser::new(code, ParserOptions::new(api::LANGUAGE))
            .parse_unsafe()
            .ast;
        let mut tokens = Vec::new();
        let mut end = 0;
        let mut after_dot = false;

        let is_library = |name: &str| greyscript_meta::is_api_function(name);

        for token in Highlighter::new(&ast)
            .with_library(&is_library)
            .tokens(api::LANGUAGE)
        {
            let (start, stop) = (token.span.start as usize, token.span.end as usize);

            if start < end || stop > code.len() || !code.is_char_boundary(start) {
                break;
            }

            Self::push(&mut tokens, &code[end..start], Highlight::Plain, None);

            let text = &code[start..stop];
            let highlight = match Self::highlight(token.kind) {
                Highlight::Name if token.library => Highlight::Library,
                highlight => highlight,
            };
            let link = (matches!(highlight, Highlight::Name | Highlight::Library) && !after_dot)
                .then(|| self.general_function(text))
                .flatten();

            Self::push(&mut tokens, text, highlight, link);
            if token.kind != HighlightKind::Comment {
                after_dot = text == ".";
            }

            end = stop;
        }

        Self::push(&mut tokens, &code[end..], Highlight::Plain, None);

        tokens
    }

    fn highlight(kind: HighlightKind) -> Highlight {
        match kind {
            HighlightKind::Keyword => Highlight::Keyword,
            HighlightKind::String => Highlight::String,
            HighlightKind::Number => Highlight::Number,
            HighlightKind::Constant => Highlight::Constant,
            HighlightKind::Comment => Highlight::Comment,
            HighlightKind::Variable
            | HighlightKind::Property
            | HighlightKind::Function
            | HighlightKind::Parameter => Highlight::Name,
            HighlightKind::Type => Highlight::Type,
            HighlightKind::Operator | HighlightKind::Punctuator => Highlight::Operator,
        }
    }

    /// Links general function `name` to its member, or returns `None` if the API omits it.
    fn general_function(&self, name: &str) -> Option<Link> {
        let general = self.general?;

        general
            .definitions()
            .contains_key(name)
            .then(|| Link::Member {
                owner: general.type_name().to_owned(),
                name: name.to_owned(),
            })
    }

    /// Appends `text` and merges adjacent plain tokens.
    fn push(tokens: &mut Vec<Token>, text: &str, highlight: Highlight, link: Option<Link>) {
        if text.is_empty() {
            return;
        }

        if highlight == Highlight::Plain
            && let Some(last) = tokens.last_mut()
            && last.highlight == Highlight::Plain
        {
            last.text.push_str(text);

            return;
        }

        tokens.push(Token {
            text: text.to_owned(),
            highlight,
            link,
        });
    }
}
