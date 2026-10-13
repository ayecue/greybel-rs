use greybel_core::{Lexer, TokenKind, Version};
use meta_utils::{BaseType, Signature};

use crate::api;
use crate::protocol::{Highlight, Link, Token};

/// Lexes GreyScript examples and links general API function calls.
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

    /// Highlights `code`; the first lexer error and all remaining source are emitted as plain text.
    pub(crate) fn code(&self, code: &str) -> Vec<Token> {
        let lexer = Lexer::new(code, api::LANGUAGE, Version::Latest);
        let mut tokens = Vec::new();
        let mut end = 0;
        let mut after_dot = false;

        for token in lexer.tokens() {
            let Ok(token) = token else { break };
            let span = token.span();
            let (start, stop) = (span.start as usize, span.end as usize);

            if start < end || stop > code.len() || !code.is_char_boundary(start) {
                break;
            }

            Self::push(&mut tokens, &code[end..start], Highlight::Plain, None);

            let text = &code[start..stop];
            let highlight = Self::highlight(token.kind());
            let link = (highlight == Highlight::Name && !after_dot)
                .then(|| self.general_function(text))
                .flatten();

            Self::push(&mut tokens, text, highlight, link);
            if token.kind() != TokenKind::Comment {
                after_dot = text == ".";
            }

            end = stop;
        }

        Self::push(&mut tokens, &code[end..], Highlight::Plain, None);

        tokens
    }

    fn highlight(kind: TokenKind) -> Highlight {
        match kind {
            TokenKind::Keyword => Highlight::Keyword,
            TokenKind::StringLiteral => Highlight::String,
            TokenKind::NumericLiteral => Highlight::Number,
            TokenKind::BooleanLiteral | TokenKind::NilLiteral => Highlight::Constant,
            TokenKind::Comment => Highlight::Comment,
            TokenKind::Identifier => Highlight::Name,
            TokenKind::Punctuator | TokenKind::SliceOperator => Highlight::Operator,
            TokenKind::Eof | TokenKind::Eol | TokenKind::Invalid => Highlight::Plain,
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
