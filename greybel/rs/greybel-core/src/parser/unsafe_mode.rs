use super::Parser;
use crate::error::ParseResult;

impl Parser<'_> {
    pub(crate) fn parse_statement(&mut self) -> ParseResult<()> {
        self.stream.statement = Some(self.token().span().start);

        if !self.stream.is_unsafe() {
            self.parse_statement_inner()?;

            return self.check_depth();
        }

        self.stream.diagnostics.begin_statement();

        let snapshot = self.stream.snapshot();

        self.parse_statement_inner()?;
        self.check_depth()?;

        if self.stream.diagnostics.has_statement_errors() {
            self.stream.diagnostics.commit_statement_failure();
            self.stream.restore(snapshot);
            while !self.token().is_eol() && !self.token().is_eof() {
                self.advance()?;
            }
        }

        Ok(())
    }
}
