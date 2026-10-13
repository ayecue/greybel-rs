/// Declaration documentation lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentationRef(Vec<String>);

impl DocumentationRef {
    pub(crate) fn push(&mut self, line: String) {
        self.0.push(line);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.0
    }
}

/// One-based definition source position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}
