use crate::error::{ParseError, ParseResult};

#[derive(Debug, Clone, Default)]
pub struct LexerErrorStorage {
    errors: Vec<ParseError>,
    unsafe_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexerErrorSnapshot(usize);

impl LexerErrorStorage {
    #[must_use]
    pub fn new_unsafe() -> Self {
        Self {
            errors: Vec::new(),
            unsafe_mode: true,
        }
    }

    #[must_use]
    pub fn is_unsafe(&self) -> bool {
        self.unsafe_mode
    }

    pub fn report(&mut self, error: ParseError) -> ParseResult<()> {
        if !self.unsafe_mode {
            return Err(Box::new(error));
        }

        self.errors.push(error);

        Ok(())
    }

    #[must_use]
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    pub fn take_errors(&mut self) -> Vec<ParseError> {
        std::mem::take(&mut self.errors)
    }

    #[must_use]
    pub fn snapshot(&self) -> LexerErrorSnapshot {
        LexerErrorSnapshot(self.errors.len())
    }

    pub fn restore(&mut self, snapshot: LexerErrorSnapshot) {
        self.errors.truncate(snapshot.0);
    }
}
