use crate::error::DefinitionError;
use crate::model::Span;

const LINE_COMMENT: &str = "//";
const DOC_COMMENT: &str = "///";
const ARROW: &str = "->";
const PUNCTUATION: &[char] = &[
    '{', '}', '(', ')', '<', '>', ',', ':', '?', '=', ';', '.', '-', '@',
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident(String),
    Text(String),
    Number(String),
    Doc(String),
    Punct(char),
    Arrow,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

pub(crate) struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    span: Span,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            span: Span { line: 1, column: 1 },
        }
    }

    pub(crate) fn tokens(mut self) -> Result<Vec<Token>, DefinitionError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next()?;
            let end = token.kind == TokenKind::End;

            tokens.push(token);
            if end {
                return Ok(tokens);
            }
        }
    }

    fn remainder(&self) -> &'a str {
        &self.source[self.offset..]
    }

    fn peek(&self) -> Option<char> {
        self.remainder().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let char = self.peek()?;

        self.offset += char.len_utf8();
        if char == '\n' {
            self.span.line += 1;
            self.span.column = 1;
        } else {
            self.span.column += 1;
        }

        Some(char)
    }

    fn bump_while(&mut self, accepts: impl Fn(char) -> bool) -> &'a str {
        let start = self.offset;

        while self.peek().is_some_and(&accepts) {
            self.bump();
        }

        &self.source[start..self.offset]
    }

    fn next(&mut self) -> Result<Token, DefinitionError> {
        loop {
            self.bump_while(char::is_whitespace);

            let span = self.span;
            let remainder = self.remainder();
            let kind = if remainder.starts_with(DOC_COMMENT)
                && !remainder[DOC_COMMENT.len()..].starts_with('/')
            {
                let line = self.bump_while(|char| char != '\n');
                let text = &line[DOC_COMMENT.len()..];

                TokenKind::Doc(text.strip_prefix(' ').unwrap_or(text).trim_end().to_owned())
            } else if remainder.starts_with(LINE_COMMENT) {
                self.bump_while(|char| char != '\n');
                continue;
            } else if remainder.starts_with(ARROW) {
                self.bump();
                self.bump();

                TokenKind::Arrow
            } else {
                match self.peek() {
                    None => TokenKind::End,
                    Some('"') => TokenKind::Text(self.text(span)?),
                    Some(char) if char.is_ascii_alphabetic() || char == '_' => {
                        let ident =
                            self.bump_while(|char| char.is_ascii_alphanumeric() || char == '_');

                        TokenKind::Ident(ident.to_owned())
                    }
                    Some(char) if char.is_ascii_digit() => {
                        let digits = self.bump_while(|char| char.is_ascii_alphanumeric());

                        TokenKind::Number(digits.to_owned())
                    }
                    Some(char) if PUNCTUATION.contains(&char) => {
                        self.bump();

                        TokenKind::Punct(char)
                    }
                    Some(char) => {
                        return Err(DefinitionError::new(
                            span,
                            format!("unexpected character `{char}`"),
                        ));
                    }
                }
            };

            return Ok(Token { kind, span });
        }
    }

    fn text(&mut self, span: Span) -> Result<String, DefinitionError> {
        self.bump();

        let mut text = String::new();

        loop {
            match self.bump() {
                Some('"') => return Ok(text),
                Some('\\') => match self.bump() {
                    Some(escaped_char @ ('"' | '\\')) => text.push(escaped_char),
                    _ => {
                        return Err(DefinitionError::new(
                            span,
                            "a text escapes only `\"` and `\\`",
                        ));
                    }
                },
                Some('\n') | None => {
                    return Err(DefinitionError::new(span, "a text isn't closed"));
                }
                Some(char) => text.push(char),
            }
        }
    }
}
