#[derive(Debug, Clone)]
pub(crate) struct FixedValueStack<T: Copy> {
    items: Vec<T>,
    index: usize,
    growable: bool,
}

impl<T: Copy> FixedValueStack<T> {
    pub(crate) fn with_capacity(capacity: usize, default: T) -> Self {
        Self::new(capacity, default, false)
    }

    pub(crate) fn growable(default: T) -> Self {
        Self::new(4, default, true)
    }

    fn new(capacity: usize, default: T, growable: bool) -> Self {
        Self {
            items: vec![default; capacity],
            index: 1,
            growable,
        }
    }

    pub(crate) fn add(&mut self, item: T) {
        if self.index < self.items.len() {
            self.items[self.index] = item;
            self.index += 1;
        } else if self.growable {
            self.items.push(item);
            self.index += 1;
        }
    }

    pub(crate) fn remove(&mut self) -> T {
        self.index = self.index.saturating_sub(1);
        if self.index == 0 {
            self.index = 1;

            return self.items[0];
        }

        self.items[self.index - 1]
    }

    pub(crate) fn peek(&self) -> T {
        self.items[self.index.max(1) - 1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CountedStyle {
    Bold,
    Italic,
    Underline,
    Uppercase,
    Lowercase,
    Strikethrough,
    Superscript,
    Subscript,
    Highlight,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FontStyleCountTable([u8; CountedStyle::COUNT]);

impl CountedStyle {
    const COUNT: usize = 9;
}

impl FontStyleCountTable {
    pub(crate) fn add(&mut self, style: CountedStyle) {
        let count = &mut self.0[style as usize];

        *count = count.wrapping_add(1);
    }

    pub(crate) fn remove(&mut self, style: CountedStyle) -> u8 {
        let count = &mut self.0[style as usize];

        *count = count.saturating_sub(1);

        *count
    }
}
