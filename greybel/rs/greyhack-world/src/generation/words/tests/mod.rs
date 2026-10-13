#[path = "support/mod.rs"]
mod support;

use greyhack_world_assets::NameList;
use support::{COUNT, words};

use super::{Gender, Word};
use crate::Rng;

#[test]
fn should_make_the_same_words_from_the_same_seed() {
    let seed = 42;

    let (first, second) = (words(Word::Surname, seed), words(Word::Surname, seed));

    assert_eq!(first, second);
}

#[test]
fn should_capitalize_made_up_names_and_keep_other_words_as_learned() {
    let kinds = [
        Word::Surname,
        Word::Corporation,
        Word::Username,
        Word::LibraryVariable,
    ];

    let lower_case_starts: Vec<String> = kinds
        .iter()
        .flat_map(|kind| words(*kind, 1))
        .filter(|word| word.chars().next().is_none_or(char::is_lowercase))
        .collect();

    assert!(lower_case_starts.is_empty(), "{lower_case_starts:?}");
}

#[test]
fn should_lower_case_about_half_of_the_passwords() {
    let passwords = words(Word::Password, 3);

    let lower = passwords
        .iter()
        .filter(|password| password.chars().all(|letter| !letter.is_uppercase()))
        .count();

    assert!(
        (COUNT / 4..COUNT * 3 / 4).contains(&lower),
        "{lower} of {COUNT}"
    );
}

#[test]
fn should_trim_quotes_and_dashes_from_both_ends() {
    let kinds = [Word::Surname, Word::Password];

    let untrimmed: Vec<String> = kinds
        .iter()
        .flat_map(|kind| words(*kind, 5))
        .filter(|word| word.starts_with(['\'', '-']) || word.ends_with(['\'', '-']))
        .collect();

    assert!(untrimmed.is_empty(), "{untrimmed:?}");
}

#[test]
fn should_pick_first_names_from_the_games_lists() {
    let mut rng = Rng::new(9);

    let names: Vec<(Gender, String)> = (0..COUNT)
        .map(|_| {
            let gender = Gender::pick(&mut rng);

            (gender, gender.first_name(&mut rng))
        })
        .collect();

    let unlisted: Vec<&(Gender, String)> = names
        .iter()
        .filter(|(gender, name)| {
            let list = match gender {
                Gender::Female => NameList::FemaleNames,
                Gender::Male => NameList::MaleNames,
            };

            name.is_empty()
                || !name.chars().next().is_some_and(char::is_uppercase)
                || !list.entries().iter().any(|entry| {
                    entry.eq_ignore_ascii_case(name) || entry.to_lowercase() == name.to_lowercase()
                })
        })
        .collect();

    assert!(unlisted.is_empty(), "{unlisted:?}");
}
