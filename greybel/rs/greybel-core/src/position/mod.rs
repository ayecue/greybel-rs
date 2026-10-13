mod line_index;
mod span;

use std::fmt;

pub use line_index::{ColumnSettings, LineIndex, PositionEncoding};
pub use span::Span;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    #[must_use]
    pub const fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.character)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    #[must_use]
    pub const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}
