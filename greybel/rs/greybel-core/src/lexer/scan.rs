use super::{
    LexResult, Lexer, PendingToken, TokenStart, TokenStream, is_identifier_part,
    is_identifier_start, is_whitespace,
};
use crate::error::ParseResult;

impl<'src> Lexer<'src> {
    pub fn advance(&self, stream: &mut TokenStream) -> ParseResult<()> {
        let token = self.next_token(stream)?;

        stream.lookahead.push(token);

        Ok(())
    }

    pub fn next_token(&self, stream: &mut TokenStream) -> LexResult {
        let old_index = stream.cursor.index();

        self.skip_whitespace(stream);

        let pending_token =
            PendingToken::new(stream.cursor.index(), old_index < stream.cursor.index());

        if self.is_at_end(stream) {
            return Ok(self.scan_eof(stream, pending_token));
        }

        match self.token_start(self.code_at(stream, 0)) {
            TokenStart::LineBreak => Ok(self.scan_eol(stream, pending_token)),
            TokenStart::Identifier => self.scan_identifier_or_keyword(stream, pending_token),
            TokenStart::Quote => self.scan_string(stream, pending_token),
            TokenStart::Digit => self.scan_number(stream, pending_token),
            TokenStart::Dot if self.code_at(stream, 1).is_ascii_digit() => {
                self.scan_number(stream, pending_token)
            }
            TokenStart::Colon => Ok(self.scan_slice_operator(stream, pending_token)),
            TokenStart::Slash if self.is_at_comment(stream) => {
                self.scan_comment(stream, pending_token)
            }
            TokenStart::Dot | TokenStart::Slash | TokenStart::Punct | TokenStart::Invalid => {
                self.scan_punct_or_invalid(stream, pending_token)
            }
        }
    }

    #[must_use]
    pub fn is_at_identifier(&self, stream: &TokenStream) -> bool {
        let c = self.code_at(stream, 0);

        is_identifier_start(c) || (self.grammar().lexer.hash_identifiers && c == b'#')
    }

    #[must_use]
    pub fn code_at(&self, stream: &TokenStream, offset: usize) -> u8 {
        self.source
            .as_bytes()
            .get(stream.cursor.index() + offset)
            .copied()
            .unwrap_or(0)
    }

    #[must_use]
    pub fn is_at_end(&self, stream: &TokenStream) -> bool {
        stream.cursor.index() >= self.source.len()
    }

    #[must_use]
    pub fn is_at_whitespace(&self, stream: &TokenStream) -> bool {
        is_whitespace(self.code_at(stream, 0))
    }

    #[must_use]
    pub fn starts_with(&self, stream: &TokenStream, text: &str) -> bool {
        self.source.as_bytes()[stream.cursor.index()..].starts_with(text.as_bytes())
    }

    pub fn starts_with_word(&self, stream: &TokenStream, text: &str) -> bool {
        let tail = &self.source.as_bytes()[stream.cursor.index()..];

        tail.starts_with(text.as_bytes())
            && !tail
                .get(text.len())
                .copied()
                .is_some_and(is_identifier_part)
    }

    fn skip_whitespace(&self, stream: &mut TokenStream) {
        let start = stream.cursor.index();
        let whitespace_len = self.source.as_bytes().get(start..).map_or(0, |remainder| {
            remainder
                .iter()
                .take_while(|&&byte| is_whitespace(byte))
                .count()
        });

        stream.cursor.set(start + whitespace_len);
    }

    fn scan_punct_or_invalid(
        &self,
        stream: &mut TokenStream,
        token_start: PendingToken,
    ) -> LexResult {
        if let Some(punct) = self.punct_at(&self.source.as_bytes()[stream.cursor.index()..]) {
            return Ok(self.scan_punct(stream, punct, token_start));
        }

        Ok(self.scan_unknown(stream, token_start))
    }
}
