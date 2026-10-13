const OPEN: u32 = u32::MAX;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn empty(offset: u32) -> Self {
        Self::new(offset, offset)
    }

    #[must_use]
    pub const fn open(&self) -> Span {
        Span::new(self.start, OPEN)
    }

    #[must_use]
    pub const fn to(&self, end: &Span) -> Span {
        Span::new(self.start, end.end)
    }

    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.end != OPEN
    }

    #[must_use]
    pub fn bytes(&self) -> Option<std::ops::Range<usize>> {
        self.is_closed()
            .then_some(self.start as usize..self.end as usize)
    }
}
