use super::*;
use crate::Rng;

pub const COUNT: usize = 200;

pub fn words(word: Word, seed: u64) -> Vec<String> {
    let mut rng = Rng::new(seed);

    (0..COUNT).map(|_| word.generate(&mut rng)).collect()
}
