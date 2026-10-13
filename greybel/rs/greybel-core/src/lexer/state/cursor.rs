#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cursor {
    index: usize,
}

impl Cursor {
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn advance(&mut self, count: usize) {
        self.index += count;
    }

    pub fn set(&mut self, index: usize) {
        self.index = index;
    }
}
