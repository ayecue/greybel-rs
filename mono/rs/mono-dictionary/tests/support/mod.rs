#![allow(dead_code)]

use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

use mono_dictionary::Dictionary;
use serde_json::Value as Json;

type Hasher = BuildHasherDefault<DefaultHasher>;

pub const KEYS: [&str; 12] = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];

pub fn hash(key: &str) -> u64 {
    BuildHasherDefault::<DefaultHasher>::default().hash_one(key)
}

pub fn dictionary(keys: &[&'static str]) -> Dictionary<&'static str, usize> {
    let mut dictionary = Dictionary::new();

    for (value, key) in keys.iter().enumerate() {
        dictionary.insert_unique(hash(key), *key, value);
    }

    dictionary
}

pub fn remove(dictionary: &mut Dictionary<&'static str, usize>, key: &str) -> Option<usize> {
    let slot = dictionary.find(hash(key), |other| *other == key)?;

    dictionary.remove_at(slot).map(|(_, value)| value)
}

pub fn keys(dictionary: &Dictionary<&'static str, usize>) -> Vec<&'static str> {
    dictionary.keys().copied().collect()
}

/// `a`, `b`, `c`, `d` with `b` and then `d` removed, and the slot `b` had.
pub fn without_b_and_d() -> (Dictionary<&'static str, usize>, usize) {
    let mut map = dictionary(&["a", "b", "c", "d"]);
    let b = map.find(hash("b"), |key| *key == "b").expect("b");

    remove(&mut map, "b");
    remove(&mut map, "d");

    (map, b)
}

/// The first four `KEYS` with `c` replaced by `x`, grown by the other eight.
pub fn grown() -> Dictionary<&'static str, usize> {
    let mut map = dictionary(&KEYS[..4]);

    remove(&mut map, "c");
    map.insert_unique(hash("x"), "x", 20);
    for (value, key) in KEYS[4..].iter().enumerate() {
        map.insert_unique(hash(key), *key, value + 4);
    }

    map
}

pub fn number(value: &Json) -> i64 {
    value.as_i64().expect("an integer")
}

/// Applies one recorded change; `false` when the dictionary responded
/// differently than the game.
pub fn apply(dictionary: &mut Dictionary<i64, i64>, change: &[Json]) -> bool {
    let hash = |key: i64| Hasher::default().hash_one(key);

    match change[0].as_str().expect("a change") {
        "clear" => {
            dictionary.clear();

            true
        }
        "remove" => {
            let key = number(&change[1]);
            let removed = dictionary
                .find(hash(key), |other| *other == key)
                .and_then(|slot| dictionary.remove_at(slot));

            removed.is_some() == change[2].as_bool().expect("a bool")
        }
        "set" => {
            let (key, value) = (number(&change[1]), number(&change[2]));

            match dictionary.find(hash(key), |other| *other == key) {
                Some(slot) => {
                    *dictionary.get_at_mut(slot).expect("a found slot").1 = value;
                }
                None => {
                    dictionary.insert_unique(hash(key), key, value);
                }
            }

            true
        }
        other => panic!("unknown change {other}"),
    }
}

/// One replayed change and what the dictionary contained after it.
pub struct Step {
    pub number_of_series: usize,
    pub step: usize,
    pub change: Vec<Json>,
    pub applied: bool,
    pub expected: Vec<(i64, i64)>,
    pub actual: Vec<(i64, i64)>,
    pub len: usize,
    pub indexed: Vec<Option<(i64, i64)>>,
    pub past_the_end: Option<(i64, i64)>,
}

/// Replays every recorded series on a fresh dictionary each.
pub fn replay(series: &[Json]) -> Vec<Step> {
    let mut steps = Vec::new();

    for (number_of_series, series) in series.iter().enumerate() {
        let mut dictionary = Dictionary::new();

        for (step, recorded) in series["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .enumerate()
        {
            let change = recorded["change"].as_array().expect("a change");
            let applied = apply(&mut dictionary, change);
            let expected: Vec<(i64, i64)> = recorded["entries"]
                .as_array()
                .expect("entries")
                .iter()
                .map(|entry| (number(&entry[0]), number(&entry[1])))
                .collect();
            let actual: Vec<(i64, i64)> = dictionary.iter().map(|(k, v)| (*k, *v)).collect();
            let indexed = (0..expected.len())
                .map(|position| dictionary.get_index(position).map(|(k, v)| (*k, *v)))
                .collect();

            steps.push(Step {
                number_of_series,
                step,
                change: change.clone(),
                applied,
                len: dictionary.len(),
                past_the_end: dictionary.get_index(expected.len()).map(|(k, v)| (*k, *v)),
                expected,
                actual,
                indexed,
            });
        }
    }

    steps
}
