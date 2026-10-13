/// Associates generated lines `first..=last` with consecutive source lines beginning at `line` in source index
/// `source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub first: u32,
    pub last: u32,
    pub source: u32,
    pub line: u32,
}

impl Mapping {
    #[must_use]
    pub const fn new(first: u32, last: u32, source: u32, line: u32) -> Self {
        Self {
            first,
            last,
            source,
            line,
        }
    }

    /// Creates a mapping from `output_line` to `line` in `source`.
    pub(crate) const fn single(output_line: u32, source: u32, line: u32) -> Self {
        Self::new(output_line, output_line, source, line)
    }

    pub(crate) fn contains(&self, output_line: u32) -> bool {
        (self.first..=self.last).contains(&output_line)
    }

    /// Resolves generated line `output_line` within this mapping to its source line.
    pub(crate) fn line_of(&self, output_line: u32) -> u32 {
        self.line + (output_line - self.first)
    }

    /// Checks whether `output_line` can extend this mapping using `line` of `source`.
    pub(crate) fn continues_at(&self, output_line: u32, source: u32, line: u32) -> bool {
        self.source == source && self.last + 1 == output_line && self.line_of(self.last) + 1 == line
    }

    /// Extracts the mapping segment preceding generated line `output_line`.
    pub(crate) fn before(&self, output_line: u32) -> Option<Self> {
        (self.first < output_line).then(|| Self {
            last: self.last.min(output_line - 1),
            ..*self
        })
    }

    /// Extracts the mapping segment following generated line `output_line`.
    pub(crate) fn after(&self, output_line: u32) -> Option<Self> {
        (self.last > output_line).then(|| {
            let first = self.first.max(output_line + 1);

            Self {
                first,
                line: self.line_of(first),
                ..*self
            }
        })
    }

    /// Shifts every generated line number in this mapping upward by `lines`.
    pub(crate) fn down_by(&self, lines: u32) -> Self {
        Self {
            first: self.first + lines,
            last: self.last + lines,
            ..*self
        }
    }
}
