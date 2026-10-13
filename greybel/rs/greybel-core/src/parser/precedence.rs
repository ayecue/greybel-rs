pub use greybel_symbols::Precedence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecedenceTable {
    levels: Vec<Precedence>,
    ranks: [u8; Precedence::COUNT],
}

impl PrecedenceTable {
    #[must_use]
    pub fn new(levels: &[Precedence]) -> Self {
        Self::from_levels(levels.to_vec())
    }

    fn from_levels(levels: Vec<Precedence>) -> Self {
        let mut ranks = [0; Precedence::COUNT];

        for (index, level) in levels.iter().enumerate() {
            ranks[level.index()] = u8::try_from(index + 1).expect("at most 254 precedence levels");
        }

        Self { levels, ranks }
    }

    #[must_use]
    pub fn insert_after(mut self, anchor: Precedence, levels: &[Precedence]) -> Self {
        let at = self.position(anchor) + 1;

        self.levels.splice(at..at, levels.iter().copied());

        Self::from_levels(self.levels)
    }

    #[must_use]
    pub fn swap(mut self, a: Precedence, b: Precedence) -> Self {
        let (a, b) = (self.position(a), self.position(b));

        self.levels.swap(a, b);

        Self::from_levels(self.levels)
    }

    #[must_use]
    pub fn levels(&self) -> &[Precedence] {
        &self.levels
    }

    #[must_use]
    pub fn rank(&self, level: Precedence) -> Option<u8> {
        Some(self.ranks[level.index()]).filter(|&rank| rank != 0)
    }

    fn position(&self, level: Precedence) -> usize {
        self.levels
            .iter()
            .position(|&l| l == level)
            .unwrap_or_else(|| panic!("precedence level `{level}` is not in the table"))
    }
}
