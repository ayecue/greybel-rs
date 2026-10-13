//! `Dictionary` against `IndexMap` on what scripts do with maps; run with `cargo bench -p mono-dictionary`.

use std::hash::{BuildHasher, BuildHasherDefault};
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use indexmap::IndexMap;
use mono_dictionary::Dictionary;
use rapidhash::fast::RapidHasher;

type Hasher = BuildHasherDefault<RapidHasher<'static>>;
type Indexed = IndexMap<u64, u64, Hasher>;

const SIZES: &[usize] = &[2, 4, 8, 16, 1024];

fn key(number: usize) -> u64 {
    (number as f64).to_bits()
}

fn hash(key: u64) -> u64 {
    Hasher::default().hash_one(key)
}

fn dictionary(size: usize) -> Dictionary<u64, u64> {
    let mut dictionary = Dictionary::new();

    for number in 0..size {
        let key = key(number);

        dictionary.insert_unique(hash(key), key, key);
    }

    dictionary
}

fn indexed(size: usize) -> Indexed {
    (0..size).map(|number| (key(number), key(number))).collect()
}

/// Sets each key as `map[key] = value` does.
fn set(dictionary: &mut Dictionary<u64, u64>, key: u64, value: u64) {
    let hash = hash(key);

    match dictionary.find(hash, |other| *other == key) {
        Some(slot) => {
            if let Some((_, old)) = dictionary.get_at_mut(slot) {
                *old = value;
            }
        }
        None => {
            dictionary.insert_unique(hash, key, value);
        }
    }
}

fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");

    for &size in SIZES {
        group.bench_with_input(BenchmarkId::new("dictionary", size), &size, |b, &size| {
            b.iter(|| {
                let mut dictionary = Dictionary::new();

                for number in 0..size {
                    set(&mut dictionary, key(number), 1);
                }

                black_box(dictionary)
            });
        });
        group.bench_with_input(BenchmarkId::new("indexmap", size), &size, |b, &size| {
            b.iter(|| {
                let mut map = Indexed::default();

                for number in 0..size {
                    map.insert(key(number), 1);
                }

                black_box(map)
            });
        });
    }

    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("get");

    for &size in SIZES {
        let (dictionary, map) = (dictionary(size), indexed(size));

        let keys: Vec<u64> = (0..size * 2).map(key).collect();

        group.bench_function(BenchmarkId::new("dictionary", size), |b| {
            b.iter(|| {
                keys.iter()
                    .filter_map(|&key| dictionary.get(hash(key), |other| *other == key))
                    .count()
            });
        });
        group.bench_function(BenchmarkId::new("indexmap", size), |b| {
            b.iter(|| keys.iter().filter_map(|key| map.get(key)).count());
        });
    }

    group.finish();
}

fn bench_overwrite(c: &mut Criterion) {
    let mut group = c.benchmark_group("overwrite");

    for &size in SIZES {
        let keys: Vec<u64> = (0..size).map(key).collect();
        let (mut dictionary, mut map) = (dictionary(size), indexed(size));

        group.bench_function(BenchmarkId::new("dictionary", size), |b| {
            b.iter(|| {
                for &key in &keys {
                    set(&mut dictionary, key, 2);
                }
            });
        });
        group.bench_function(BenchmarkId::new("indexmap", size), |b| {
            b.iter(|| {
                for &key in &keys {
                    map.insert(key, 2);
                }
            });
        });
    }

    group.finish();
}

/// `for entry in map`: the interpreter reads entry `i` by position.
fn bench_positions(c: &mut Criterion) {
    let mut group = c.benchmark_group("positions");

    for &size in SIZES {
        let (dictionary, map) = (dictionary(size), indexed(size));

        group.bench_function(BenchmarkId::new("dictionary", size), |b| {
            b.iter(|| {
                (0..size)
                    .filter_map(|position| dictionary.get_index(position))
                    .map(|(_, value)| *value)
                    .sum::<u64>()
            });
        });
        group.bench_function(BenchmarkId::new("indexmap", size), |b| {
            b.iter(|| {
                (0..size)
                    .filter_map(|position| map.get_index(position))
                    .map(|(_, value)| *value)
                    .sum::<u64>()
            });
        });
    }

    group.finish();
}

fn bench_iterate(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate");

    for &size in SIZES {
        let (dictionary, map) = (dictionary(size), indexed(size));
        let mut vacated = self::dictionary(size + size / 4);

        for number in size..size + size / 4 {
            let key = key(number);
            let slot = vacated.find(hash(key), |other| *other == key);

            vacated.remove_at(slot.expect("a key"));
        }

        group.bench_function(BenchmarkId::new("dictionary", size), |b| {
            b.iter(|| dictionary.values().sum::<u64>());
        });
        group.bench_function(BenchmarkId::new("dictionary-vacated", size), |b| {
            b.iter(|| vacated.values().sum::<u64>());
        });
        group.bench_function(BenchmarkId::new("indexmap", size), |b| {
            b.iter(|| map.values().sum::<u64>());
        });
    }

    group.finish();
}

/// Removing a key and adding another, so the dictionary retains its size.
fn bench_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("churn");

    for &size in SIZES {
        let (mut dictionary, mut map) = (dictionary(size), indexed(size));

        let mut next = size;

        group.bench_function(BenchmarkId::new("dictionary", size), |b| {
            b.iter(|| {
                let old = key(next - size);
                let slot = dictionary.find(hash(old), |other| *other == old);

                dictionary.remove_at(slot.expect("a key"));
                set(&mut dictionary, key(next), 1);
                next += 1;
            });
        });

        let mut next = size;

        group.bench_function(BenchmarkId::new("indexmap", size), |b| {
            b.iter(|| {
                map.shift_remove(&key(next - size)).expect("a key");
                map.insert(key(next), 1);
                next += 1;
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_build,
    bench_get,
    bench_overwrite,
    bench_positions,
    bench_iterate,
    bench_churn
);

criterion_main!(benches);
