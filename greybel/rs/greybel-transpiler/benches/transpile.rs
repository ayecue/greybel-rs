//! Transpiling throughput on the MIT-licensed MiniScript corpus in `greybel/benches/corpus`.

use std::hint::black_box;
use std::path::Path;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use greybel_documents::Uri;
use greybel_transpiler::{BeautifyOptions, BuildMode, TranspileOptions, Transpiler, UglifyOptions};

const CORPUS: &[&str] = &[
    "superstartrek.ms",
    "textAdventure.ms",
    "therapist.ms",
    "tetris.ms",
    "vt.ms",
    "tsv.ms",
    "acey-deucey.ms",
];

fn corpus() -> Vec<String> {
    let dir = Path::new(env!("BENCH_CORPUS"));

    CORPUS
        .iter()
        .map(|name| std::fs::read_to_string(dir.join(name)).expect("corpus file"))
        .collect()
}

fn bench(c: &mut Criterion) {
    let files = corpus();
    let total: usize = files.iter().map(String::len).sum();
    let modes = [
        ("parse", None),
        ("default", Some(BuildMode::Default)),
        ("uglify", Some(BuildMode::Uglify(UglifyOptions::default()))),
        (
            "beautify",
            Some(BuildMode::Beautify(BeautifyOptions::default())),
        ),
    ];
    let mut group = c.benchmark_group("corpus");

    group.throughput(Throughput::Bytes(total as u64));
    for (name, mode) in modes {
        let options = TranspileOptions {
            mode: mode.clone().unwrap_or_default(),
            ..TranspileOptions::default()
        };

        for source in &files {
            Transpiler::new(&options)
                .transpile(source)
                .expect("corpus transpiles");
        }

        group.bench_function(name, |b| {
            b.iter(|| {
                for source in &files {
                    match mode {
                        None => drop(black_box(
                            Transpiler::new(&options).parse(&Uri::memory("x"), source),
                        )),
                        Some(_) => drop(black_box(Transpiler::new(&options).transpile(source))),
                    }
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
