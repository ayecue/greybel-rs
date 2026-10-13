use crate::error::{ParseError, ParseResult};

#[derive(Debug, Default)]
pub struct ParseErrorStorage {
    unsafe_mode: bool,
    errors: Vec<ParseError>,
    statement_errors: Vec<ParseError>,
}

impl ParseErrorStorage {
    pub(crate) fn new_unsafe() -> Self {
        Self {
            unsafe_mode: true,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn is_unsafe(&self) -> bool {
        self.unsafe_mode
    }

    pub fn report(&mut self, error: ParseError) -> ParseResult<()> {
        if self.unsafe_mode {
            self.statement_errors.push(error);

            Ok(())
        } else {
            Err(Box::new(error))
        }
    }

    pub(crate) fn report_global(&mut self, error: ParseError) -> ParseResult<()> {
        if self.unsafe_mode {
            self.errors.push(error);

            Ok(())
        } else {
            Err(Box::new(error))
        }
    }

    pub(crate) fn begin_statement(&mut self) {
        self.statement_errors.clear();
    }

    pub(crate) fn has_statement_errors(&self) -> bool {
        !self.statement_errors.is_empty()
    }

    pub(crate) fn commit_statement_failure(&mut self) {
        if !self.statement_errors.is_empty() {
            let first = self.statement_errors.swap_remove(0);

            self.statement_errors.clear();
            self.errors.push(first);
        }
    }

    pub(crate) fn take_errors(&mut self) -> Vec<ParseError> {
        std::mem::take(&mut self.errors)
    }
}
