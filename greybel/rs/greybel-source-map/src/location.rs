/// A source file and one of its 1-based lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location<'m> {
    pub source: &'m str,
    pub line: u32,
}

impl<'m> Location<'m> {
    #[must_use]
    pub const fn new(source: &'m str, line: u32) -> Self {
        Self { source, line }
    }
}
