use mono_dictionary::Dictionary;
use support::{dictionary, grown, hash, keys, remove, without_b_and_d};

use crate::support;

#[test]
fn should_return_the_value_of_a_removed_key() {
    let mut map = dictionary(&["a", "b", "c", "d"]);
    let removed = remove(&mut map, "b");

    assert_eq!(removed, Some(1));
}

#[test]
fn should_return_the_value_of_a_removed_last_key() {
    let mut map = dictionary(&["a", "b", "c", "d"]);

    remove(&mut map, "b");

    let removed = remove(&mut map, "d");

    assert_eq!(removed, Some(3));
}

#[test]
fn should_vacate_the_slots_of_removed_keys() {
    let (mut map, b) = without_b_and_d();
    let removed_again = remove(&mut map, "d");

    assert_eq!(removed_again, None);
    assert_eq!(keys(&map), ["a", "c"]);
    assert_eq!(map.get_at(b), None);
    assert_eq!(map.get_index(1), Some((&"c", &2)));
    assert_eq!(map.iter().len(), 2);
}

#[test]
fn should_add_a_new_key_into_the_slot_vacated_last() {
    let (mut map, _) = without_b_and_d();

    remove(&mut map, "d");

    let slot = map.insert_unique(hash("e"), "e", 4);

    assert_eq!(slot, 3);
}

#[test]
fn should_add_the_next_key_into_the_slot_vacated_before() {
    let (mut map, b) = without_b_and_d();

    remove(&mut map, "d");
    map.insert_unique(hash("e"), "e", 4);

    let slot = map.insert_unique(hash("f"), "f", 5);

    assert_eq!(slot, b);
}

#[test]
fn should_append_a_key_once_no_slot_is_vacant() {
    let (mut map, b) = without_b_and_d();

    remove(&mut map, "d");
    map.insert_unique(hash("e"), "e", 4);
    map.insert_unique(hash("f"), "f", 5);

    let slot = map.insert_unique(hash("g"), "g", 6);

    assert_eq!(slot, 4);
    assert_eq!(keys(&map), ["a", "f", "c", "e", "g"]);
    assert_eq!(map.get_at(b), Some((&"f", &5)));
}

#[test]
fn should_empty_a_cleared_dictionary() {
    let mut map = dictionary(&["a", "b", "c"]);

    remove(&mut map, "a");
    map.clear();
    assert!(map.is_empty());
}

#[test]
fn should_start_over_at_the_first_slot_after_clearing() {
    let mut map = dictionary(&["a", "b", "c"]);

    remove(&mut map, "a");
    map.clear();

    let slot = map.insert_unique(hash("x"), "x", 0);

    assert_eq!(slot, 0);
}

#[test]
fn should_continue_at_the_next_slot_after_clearing() {
    let mut map = dictionary(&["a", "b", "c"]);

    remove(&mut map, "a");
    map.clear();
    map.insert_unique(hash("x"), "x", 0);

    let slot = map.insert_unique(hash("y"), "y", 1);

    assert_eq!(slot, 1);
    assert_eq!(keys(&map), ["x", "y"]);
}

#[test]
fn should_not_write_a_vacant_slot() {
    let mut map = dictionary(&["a", "b"]);

    remove(&mut map, "a");

    let entry = map.get_at_mut(0);

    assert!(entry.is_none());
}

#[test]
fn should_not_read_a_slot_past_the_last() {
    let mut map = dictionary(&["a", "b"]);

    remove(&mut map, "a");

    let entry = map.get_at(2);

    assert!(entry.is_none());
}

#[test]
fn should_not_remove_a_vacant_slot() {
    let mut map = dictionary(&["a", "b"]);

    remove(&mut map, "a");

    let removed = map.remove_at(0);

    assert!(removed.is_none());
}

#[test]
fn should_not_find_a_removed_key() {
    let mut map = dictionary(&["a", "b"]);

    remove(&mut map, "a");

    let slot = map.find(hash("a"), |key| *key == "a");

    assert_eq!(slot, None);
}

#[test]
fn should_write_an_occupied_slot_after_a_vacant_one() {
    let mut map = dictionary(&["a", "b"]);

    remove(&mut map, "a");
    *map.get_at_mut(1).expect("b").1 = 9;
    assert_eq!(map.get_index(0), Some((&"b", &9)));
}

#[test]
fn should_get_the_key_with_the_greatest_hash() {
    let mut map = Dictionary::new();

    map.insert_unique(u64::MAX, "max", 1);
    map.insert_unique(u64::MAX - 1, "below", 2);

    let entry = map.get(u64::MAX, |key| *key == "max");

    assert_eq!(entry, Some((&"max", &1)));
}

#[test]
fn should_get_the_key_below_the_greatest_hash() {
    let mut map = Dictionary::new();

    map.insert_unique(u64::MAX, "max", 1);
    map.insert_unique(u64::MAX - 1, "below", 2);

    let entry = map.get(u64::MAX - 1, |key| *key == "below");

    assert_eq!(entry, Some((&"below", &2)));
}

#[test]
fn should_read_the_slot_found_for_the_greatest_hash() {
    let mut map = Dictionary::new();

    map.insert_unique(u64::MAX, "max", 1);
    map.insert_unique(u64::MAX - 1, "below", 2);

    let slot = map.find(u64::MAX, |key| *key == "max").expect("max");
    let entry = map.get_at(slot);

    assert_eq!(entry, Some((&"max", &1)));
}

#[test]
fn should_remove_the_slot_found_for_the_greatest_hash() {
    let mut map = Dictionary::new();

    map.insert_unique(u64::MAX, "max", 1);
    map.insert_unique(u64::MAX - 1, "below", 2);

    let slot = map.find(u64::MAX, |key| *key == "max").expect("max");
    let removed = map.remove_at(slot);

    assert_eq!(removed, Some(("max", 1)));
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), ["below"]);
}

#[test]
fn should_fit_a_cache_line() {
    type Map = Dictionary<u64, u64>;

    let size = size_of::<Map>();

    assert_eq!(size, 64);
}

#[test]
fn should_remove_a_key_from_a_small_dictionary() {
    let mut map = dictionary(&support::KEYS[..4]);
    let removed = remove(&mut map, "c");

    assert_eq!(removed, Some(2));
    assert_eq!(map.len(), 3);
    assert_eq!(map.find(hash("c"), |key| *key == "c"), None);
}

#[test]
fn should_reuse_the_vacated_slot_of_a_small_dictionary() {
    let mut map = dictionary(&support::KEYS[..4]);

    remove(&mut map, "c");

    let slot = map.insert_unique(hash("x"), "x", 20);

    assert_eq!(slot, 2);
}

#[test]
fn should_keep_the_order_when_a_small_dictionary_grows() {
    let mut map = dictionary(&support::KEYS[..4]);

    remove(&mut map, "c");
    map.insert_unique(hash("x"), "x", 20);
    for (value, key) in support::KEYS[4..].iter().enumerate() {
        map.insert_unique(hash(key), *key, value + 4);
    }

    assert_eq!(map.len(), 12);
    assert_eq!(
        keys(&map),
        ["a", "b", "x", "d", "e", "f", "g", "h", "i", "j", "k", "l"]
    );
}

#[test]
fn should_find_the_keys_of_a_grown_dictionary() {
    let map = grown();
    let wanted = ["a", "x", "h", "l"];
    let found: Vec<_> = wanted
        .iter()
        .map(|&key| {
            let slot = map.find(hash(key), |other| *other == key).expect(key);

            (key, map.get_at(slot).map(|(k, _)| *k))
        })
        .collect();

    for (key, found) in found {
        assert_eq!(found, Some(key));
    }
}

#[test]
fn should_remove_a_key_from_a_grown_dictionary() {
    let mut map = grown();
    let removed = remove(&mut map, "a");

    assert_eq!(removed, Some(0));
    assert_eq!(map.find(hash("a"), |key| *key == "a"), None);
}

#[test]
fn should_start_over_after_clearing_a_grown_dictionary() {
    let mut map = grown();

    remove(&mut map, "a");
    map.clear();

    let slot = map.insert_unique(hash("z"), "z", 0);

    assert_eq!(slot, 0);
    assert_eq!(map.get(hash("z"), |key| *key == "z"), Some((&"z", &0)));
}

#[test]
fn should_allocate_nothing_while_empty() {
    let map = Dictionary::<&str, usize>::new();
    let size = map.allocation_size();

    assert_eq!(size, 0);
}

#[test]
fn should_count_the_index_once_a_dictionary_outgrows_small() {
    let small = dictionary(&support::KEYS[..4]);
    let indexed = dictionary(&support::KEYS[..5]);
    let entries = indexed.capacity() * small.allocation_size() / small.capacity();

    assert!(indexed.allocation_size() > entries);
}

#[test]
fn should_grow_the_allocation_with_the_entries() {
    let few = dictionary(&support::KEYS[..6]);
    let many = dictionary(&support::KEYS);

    assert!(many.allocation_size() > few.allocation_size());
}
