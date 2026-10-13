use super::MarkovNameGenerator;
use crate::Rng;

#[test]
fn should_drop_the_first_letter_of_a_start_that_doesnt_sound_like_one() {
    let words = ["Xbcd", "Aabc", "Shan", "Brad", "Mcka", "Ebon"];

    let fixed: Vec<String> = words
        .iter()
        .map(|word| MarkovNameGenerator::fix_start(word))
        .collect();

    assert_eq!(fixed, ["Bcd", "Abc", "Shan", "Brad", "Mcka", "Ebon"]);
}

#[test]
fn should_capitalize_each_part_of_a_word_with_spaces() {
    let text: Vec<char> = "BLACK  HAT X".chars().collect();

    let word = MarkovNameGenerator::capitalize(&text);

    assert_eq!(word, "Black Hat X");
}

#[test]
fn should_learn_only_from_parts_longer_than_its_order() {
    let samples = ["ab, abcd", "xyz"];

    let generator = MarkovNameGenerator::new(samples, 3, 3);

    assert_eq!(generator.samples.len(), 1);
}

#[test]
fn should_make_words_from_the_letters_of_its_samples() {
    let generator = MarkovNameGenerator::new(["abcdef"], 3, 3);
    let mut rng = Rng::new(7);

    let words: Vec<String> = (0..20).map(|_| generator.next_name(&mut rng)).collect();

    assert!(
        words.iter().all(|word| word.chars().count() >= 3
            && word
                .to_lowercase()
                .chars()
                .all(|letter| "abcdef".contains(letter))),
        "{words:?}"
    );
}
