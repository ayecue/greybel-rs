//! Tests value formatting, equality, and maps against TypeScript interpreter snapshots.

use std::rc::Rc;

use greybel_interpreter::value::{Key, Number};
use greybel_interpreter::{Closure, FunctionBody, Heap, Param, Value};

use crate::support::value::{Equality, abc, cyclic_maps, keys, list, map};

#[test]
fn should_print_maps_in_insertion_order() {
    let mut heap = Heap::default();
    let value = map(
        &mut heap,
        vec![
            ("was".into(), 1.0.into()),
            ("foo".into(), 0.0.into()),
            ("bar".into(), "xxx".into()),
        ],
    );
    let printed = heap.display(&value).to_string();

    assert_eq!(printed, r#"{"was": 1, "foo": 0, "bar": "xxx"}"#);
}

#[test]
fn should_elide_nested_maps_after_two_levels() {
    let mut heap = Heap::default();
    let foo = heap.add_map_from(vec![
        ("was".into(), 1.0.into()),
        ("second".into(), 2.0.into()),
    ]);
    let x = map(&mut heap, vec![("foo".into(), 2.0.into())]);

    foo.set(&mut heap, x.clone(), Value::Map(foo));

    let printed = heap
        .display(&foo.get(&heap, &x).cloned().expect("set"))
        .to_string();

    assert_eq!(
        printed,
        r#"{"was": 1, "second": 2, {"foo": 2}: {"was": 1, "second": 2, {"foo": 2}: {"was": 1, "second": 2, {...}: {...}}}}"#
    );
}

#[test]
fn should_print_nested_lists() {
    let mut heap = Heap::default();
    let inner = list(&mut heap, vec!["a".into(), Value::Null]);
    let value = list(&mut heap, vec![1.0.into(), 0.5.into(), inner]);
    let printed = heap.display(&value).to_string();

    assert_eq!(printed, r#"[1, 0.5, ["a", null]]"#);
}

#[test]
fn should_print_strings_without_quotes() {
    let heap = Heap::default();
    let value: Value = "plain".into();
    let printed = heap.display(&value).to_string();

    assert_eq!(printed, "plain");
}

#[test]
fn should_print_a_cyclic_list_without_overflowing() {
    let mut heap = Heap::default();
    let id = heap.add_list(Vec::new());

    id.items_mut(&mut heap).push(Value::List(id));

    let text = heap.display(&Value::List(id)).to_string();

    assert!(text.contains("[...]"), "{text}");
    assert_eq!(text.matches('[').count(), text.matches(']').count());
}

#[test]
fn should_print_the_parameters_of_functions() {
    let mut heap = Heap::default();
    let id = heap.add_function(Closure {
        params: Rc::from(vec![
            Param::new("self"),
            Param::with_default("delimiter", ",".into()),
        ]),
        body: FunctionBody::Native {
            call: Rc::new(|_, _| Ok(Value::Null.into())),
            label: None,
        },
        outer: None,
    });
    let printed = heap.display(&Value::Function(id)).to_string();

    assert_eq!(printed, r#"FUNCTION(self, delimiter=",")"#);
}

#[test]
fn should_compare_lists_deeply() {
    let Equality {
        heap,
        list_a: a,
        list_b: b,
        ..
    } = Equality::default();
    let equal = heap.equal(&a, &b);

    assert!(equal);
}

#[test]
fn should_tell_lists_of_other_items_apart() {
    let Equality {
        heap,
        list_a: a,
        list_c: c,
        ..
    } = Equality::default();
    let equal = heap.equal(&a, &c);

    assert!(!equal);
}

#[test]
fn should_hash_equal_lists_alike() {
    let Equality {
        heap,
        list_a: a,
        list_b: b,
        ..
    } = Equality::default();
    let hashes = (heap.key_hash(&a), heap.key_hash(&b));

    assert_eq!(hashes.0, hashes.1);
}

#[test]
fn should_find_an_entry_by_an_equal_list_key() {
    let Equality {
        heap,
        list_b: b,
        table,
        ..
    } = Equality::default();
    let found = table.get(&heap, &b).map(|v| heap.display(v).to_string());

    assert_eq!(found, Some("test".to_owned()));
}

#[test]
fn should_compare_maps_regardless_of_order() {
    let Equality {
        heap,
        map_x: x,
        map_y: y,
        ..
    } = Equality::default();
    let equal = heap.equal(&x, &y);

    assert!(equal, "order doesn't matter");
}

#[test]
fn should_hash_equal_maps_alike() {
    let Equality {
        heap,
        map_x: x,
        map_y: y,
        ..
    } = Equality::default();
    let hashes = (heap.key_hash(&x), heap.key_hash(&y));

    assert_eq!(hashes.0, hashes.1);
}

#[test]
fn should_tell_a_map_from_null() {
    let Equality { heap, map_x: x, .. } = Equality::default();
    let equal = heap.equal(&x, &Value::Null);

    assert!(!equal);
}

#[test]
fn should_compare_one_equal_to_true() {
    let Equality { heap, .. } = Equality::default();
    let one: Value = 1.0.into();
    let equal = heap.equal(&one, &Value::TRUE);

    assert!(equal);
}

#[test]
fn should_compare_cyclic_maps() {
    let (heap, a, b) = cyclic_maps();
    let equal = heap.equal(&Value::Map(a), &Value::Map(b));

    assert!(equal);
}

#[test]
fn should_hash_cyclic_maps_alike() {
    let (heap, a, b) = cyclic_maps();
    let hashes = (heap.key_hash(&Value::Map(a)), heap.key_hash(&Value::Map(b)));

    assert_eq!(hashes.0, hashes.1);
}

#[test]
fn should_keep_the_place_of_a_key_set_again() {
    let mut heap = Heap::default();
    let id = heap.add_map_from(vec![("a".into(), 1.0.into()), ("b".into(), 2.0.into())]);

    id.set(&mut heap, "a".into(), 3.0.into());
    assert_eq!(
        heap.display(&Value::Map(id)).to_string(),
        r#"{"a": 3, "b": 2}"#
    );
}

#[test]
fn should_find_the_names_a_map_was_built_from() {
    let mut heap = Heap::default();
    let (a, b) = (Key::new("a"), Key::new("b"));
    let id = heap.add_map_from_keys([(&a, 1.0.into()), (&b, 2.0.into()), (&a, 3.0.into())]);

    assert_eq!(
        heap.display(&Value::Map(id)).to_string(),
        r#"{"a": 3, "b": 2}"#
    );
    assert_eq!(id.get(&heap, &"b".into()).map(Value::to_number), Some(2.0));
    assert_eq!(id.get_key(&heap, &a).map(Value::to_number), Some(3.0));
}

#[test]
fn should_find_entries_by_str() {
    let mut heap = Heap::default();
    let id = heap.add_map_from(vec![
        ("a".into(), 1.0.into()),
        (1.0.into(), 2.0.into()),
        ("".into(), 3.0.into()),
    ]);
    let found = ["a", "1", "", "b"].map(|key| id.get_str(&heap, key).map(Value::to_number));

    assert_eq!(found, [Some(1.0), None, Some(3.0), None]);
}

#[test]
fn should_drop_a_removed_key() {
    let mut heap = Heap::default();
    let id = heap.add_map_from(vec![("a".into(), 1.0.into()), ("b".into(), 2.0.into())]);

    id.set(&mut heap, "a".into(), 3.0.into());
    id.remove(&mut heap, &"a".into());
    assert_eq!(heap.display(&Value::Map(id)).to_string(), r#"{"b": 2}"#);
}

#[test]
fn should_tell_truthy_values() {
    let mut heap = Heap::default();
    let empty = list(&mut heap, Vec::new());
    let values = [empty, Value::Null, "".into(), 0.1.into()];
    let truthy = values.map(|value| heap.is_truthy(&value));

    assert!(!truthy[0]);
    assert!(!truthy[1]);
    assert!(!truthy[2]);
    assert!(truthy[3]);
}

/// As C#'s `(int)` cast in the game, which keeps the low 32 bits of the 64-bit truncation: in Grey Hack
/// `bitAnd(2147483648, 2147483648)` is `-2147483648`, `bitAnd(2^33 + 5, 3)` is `1` and `bitOr(-2147483649, 0)` is
/// `2147483647`, `bitOr(0 / 0, 0)` is `0` and `bitOr(1e30, 0)` is `-1`.
#[test]
fn should_convert_numbers_to_ints() {
    let numbers = [
        -1.7,
        2_147_483_648.0,
        8_589_934_597.0,
        -2_147_483_649.0,
        f64::NAN,
        1e30,
    ];
    let ints = numbers.map(|number| Value::from(number).to_int());

    assert_eq!(ints, [-1, i32::MIN, 5, i32::MAX, 0, -1]);
    assert_eq!(Value::Null.to_int(), 0);
}

#[test]
fn should_print_numbers_as_in_csharp() {
    let cases = [
        (123.0, "123"),
        (1.23, "1.23"),
        (1.23e9, "1230000000"),
        (-1.23e9, "-1230000000"),
        (1.23e-9, "1.230000E-09"),
        (-1.23e-9, "-1.230000E-09"),
        (3.486_784_401e29, "348678440100000000000000000000"),
        (12_345_678_901.5, "1.234568E+010"),
        (12_345_678_901_234_567.0, "12345678901234600"),
        (-1e20, "-100000000000000000000"),
        (0.1 + 0.2, "0.3"),
        (1.0 / 3.0, "0.333333"),
        (-0.0, "0"),
        (f64::INFINITY, "Infinity"),
        (f64::NAN, "NaN"),
    ];
    let printed = cases.map(|(number, _)| Number::new(number).to_string());

    for ((number, text), printed) in cases.into_iter().zip(printed) {
        assert_eq!(printed, text, "{number}");
    }
}

/// The expected hashes are what `Value.Hash()` returns on Grey Hack's own runtime.
#[test]
fn should_compute_the_hash_codes_the_game_computes() {
    let mut heap = Heap::default();
    let names = list(&mut heap, vec!["foo".into(), "bar".into()]);
    let counts = map(
        &mut heap,
        vec![("foo".into(), 2.0.into()), ("bar".into(), 0.0.into())],
    );
    let inner = map(
        &mut heap,
        vec![("myTest".into(), 0.0.into()), (1.0.into(), 2.0.into())],
    );
    let outer = map(&mut heap, vec![(inner, 1.0.into())]);
    let with_null = list(&mut heap, vec![Value::Null, 1.0.into()]);
    let null_key = map(&mut heap, vec![(Value::Null, Value::Null)]);
    let id = heap.add_list(Vec::new());

    id.items_mut(&mut heap)
        .extend([Value::List(id), "x".into()]);

    let values = [
        "foo".into(),
        1.0.into(),
        1345.0.into(),
        names,
        counts,
        outer,
        with_null,
        null_key,
        Value::List(id),
    ];
    let hashes = values.map(|value| heap.hash_code(&value));

    assert_eq!(hashes[0], 1_502_598_398);
    assert_eq!(hashes[1], 1_072_693_248);
    assert_eq!(hashes[2], 1_083_507_712);
    assert_eq!(hashes[3], -2_051_310_762);
    assert_eq!(hashes[4], -1_815_023_436);
    assert_eq!(hashes[5], 438_123_312);
    assert_eq!(hashes[6], -1_074_790_400);
    assert_eq!(hashes[7], 1_073_741_823);
    assert_eq!(hashes[8], 372_029_399);
}

/// The game's `Dictionary` reuses a removed key's slot (the last freed first); expected orders come from `ValMap` on Grey Hack's runtime.
#[test]
fn should_reuse_a_removed_keys_slot_like_the_game() {
    let (mut heap, map) = abc();

    map.remove(&mut heap, &"a".into());
    map.set(&mut heap, "d".into(), 1.0.into());
    assert_eq!(keys(&heap, map), ["d", "b", "c"]);
}

#[test]
fn should_leave_no_entries_in_the_slots_of_removed_keys() {
    let (mut heap, map) = abc();

    map.remove(&mut heap, &"a".into());
    map.set(&mut heap, "d".into(), 1.0.into());
    map.remove(&mut heap, &"b".into());
    map.remove(&mut heap, &"c".into());
    assert_eq!(keys(&heap, map), ["d"]);
    assert_eq!(map.entries(&heap).len(), 1);
    assert!(map.entries(&heap).get_index(1).is_none());
    assert!(map.get(&heap, &"b".into()).is_none());
}

#[test]
fn should_reuse_the_last_freed_slot_first_like_the_game() {
    let (mut heap, map) = abc();

    map.remove(&mut heap, &"a".into());
    map.set(&mut heap, "d".into(), 1.0.into());
    map.remove(&mut heap, &"b".into());
    map.remove(&mut heap, &"c".into());
    for key in ["e", "f", "g"] {
        map.set(&mut heap, key.into(), 1.0.into());
    }

    assert_eq!(keys(&heap, map), ["d", "f", "e", "g"]);
    assert_eq!(
        map.entries(&heap)
            .get_index(1)
            .map(|(key, _)| heap.display(key).to_string()),
        Some("f".to_owned())
    );
}

#[test]
fn should_hash_a_map_with_reused_slots_like_the_game() {
    let (mut heap, map) = abc();

    map.remove(&mut heap, &"a".into());
    map.set(&mut heap, "d".into(), 1.0.into());
    map.remove(&mut heap, &"b".into());
    map.remove(&mut heap, &"c".into());
    for key in ["e", "f", "g"] {
        map.set(&mut heap, key.into(), 1.0.into());
    }

    let hash = heap.hash_code(&Value::Map(map));

    assert_eq!(hash, -216_300_560);
}
