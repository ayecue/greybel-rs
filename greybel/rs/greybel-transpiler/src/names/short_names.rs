#[derive(Debug, Clone)]
pub struct ShortNameIter {
    characters: Vec<char>,
    counter: Vec<usize>,
}

impl ShortNameIter {
    #[must_use]
    pub fn new(characters: Vec<char>) -> Self {
        assert!(!characters.is_empty(), "short name charset is empty");

        Self {
            characters,
            counter: vec![0],
        }
    }

    pub fn write_next(&mut self, name: &mut String) {
        name.clear();

        let mut index = 0;

        while index < self.counter.len() {
            name.push(self.characters[self.counter[index]]);
            if index == self.counter.len() - 1 {
                self.increment();
            }

            index += 1;
        }
    }

    fn increment(&mut self) {
        for digit in self.counter.iter_mut().rev() {
            *digit += 1;
            if *digit < self.characters.len() {
                return;
            }

            *digit = 0;
        }

        self.counter.push(0);
    }
}

impl Iterator for ShortNameIter {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let mut name = String::new();

        self.write_next(&mut name);

        Some(name)
    }
}
