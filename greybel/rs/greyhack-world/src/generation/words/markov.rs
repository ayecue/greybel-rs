use std::collections::HashMap;

use crate::Rng;

/// Generates words using `order`-letter Markov chains trained on sample lines.
#[derive(Debug)]
pub(crate) struct MarkovNameGenerator {
    chains: HashMap<Vec<char>, Vec<char>>,
    samples: Vec<Vec<char>>,
    order: usize,
    min_length: usize,
}

impl MarkovNameGenerator {
    const NO_LETTER: char = '?';

    /// Splits samples at commas, discards parts shorter than `order + 1`, and uppercases retained
    /// parts.
    pub(crate) fn new<'a>(
        samples: impl IntoIterator<Item = &'a str>,
        order: usize,
        min_length: usize,
    ) -> Self {
        let order = order.max(1);
        let samples: Vec<Vec<char>> = samples
            .into_iter()
            .flat_map(|line| line.split(','))
            .map(|part| part.trim().to_uppercase().chars().collect::<Vec<_>>())
            .filter(|part| part.len() > order)
            .collect();
        let mut chains: HashMap<Vec<char>, Vec<char>> = HashMap::new();

        for sample in &samples {
            for start in 0..sample.len() - order {
                chains
                    .entry(sample[start..start + order].to_vec())
                    .or_default()
                    .push(sample[start + order]);
            }
        }

        Self {
            chains,
            samples,
            order,
            min_length: min_length.max(1),
        }
    }

    /// Generates a word with Grey Hack-compatible behavior: two random draws per step and
    /// first-letter loss after an invalid start.
    pub(crate) fn next_name(&self, rng: &mut Rng) -> String {
        self.next_name_with(|len| rng.index(len))
    }

    /// Generates a word using caller-provided random indices.
    pub(crate) fn next_name_with(&self, mut index: impl FnMut(usize) -> usize) -> String {
        let word = loop {
            let sample = &self.samples[index(self.samples.len())];
            let start = index(sample.len() - self.order);
            let mut text: Vec<char> = sample[start..start + self.order].to_vec();

            while text.len() < sample.len() {
                let token = &text[text.len() - self.order..];

                if self.letter_with(token, &mut index) == Self::NO_LETTER {
                    break;
                }

                let letter = self.letter_with(token, &mut index);

                text.push(letter);
            }

            let word = Self::capitalize(&text);

            if word.chars().count() >= self.min_length {
                break word;
            }
        };

        Self::fix_start(&word)
    }

    fn letter_with(&self, token: &[char], index: &mut impl FnMut(usize) -> usize) -> char {
        match self.chains.get(token) {
            Some(letters) => letters[index(letters.len())],
            None => Self::NO_LETTER,
        }
    }

    /// Capitalizes each nonempty space-separated part by keeping its first letter uppercase and
    /// lowercasing the rest.
    fn capitalize(text: &[char]) -> String {
        let text: String = text.iter().collect();

        if !text.contains(' ') {
            return Self::title(&text);
        }

        text.split(' ')
            .filter(|part| !part.is_empty())
            .map(|part| {
                if part.chars().count() == 1 {
                    part.to_uppercase()
                } else {
                    Self::title(part)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn title(text: &str) -> String {
        let mut chars = text.chars();

        chars.next().map_or_else(String::new, |first| {
            std::iter::once(first)
                .chain(chars.as_str().to_lowercase().chars())
                .collect()
        })
    }

    fn fix_start(word: &str) -> String {
        let chars: Vec<char> = word.chars().collect();
        let (first, second) = (chars[0], chars[1]);
        let pair: String = [first, second].iter().collect();
        let unchanged = !first.to_lowercase().eq(std::iter::once(second))
            && (matches!(second, 'h' | 'r' | 'l' | '\'')
                || pair == "Ch"
                || Self::is_vowel(first)
                || pair == "Mc"
                || Self::is_vowel(second)
                || first == 'S');

        if unchanged {
            word.to_owned()
        } else {
            second
                .to_uppercase()
                .chain(chars[2..].iter().copied())
                .collect()
        }
    }

    fn is_vowel(letter: char) -> bool {
        "aeiouAEIOU".contains(letter)
    }
}

#[cfg(test)]
#[path = "tests/markov.rs"]
mod tests;
