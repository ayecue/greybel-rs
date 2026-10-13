//! Lexing and parsing throughput on the MIT-licensed MiniScript corpus in `greybel/benches/corpus`.

use std::hint::black_box;
use std::path::Path;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use greybel_core::{Dialect, Lexer, Parser, ParserOptions, Version};

const CORPUS: &[&str] = &[
    "superstartrek.ms",
    "textAdventure.ms",
    "therapist.ms",
    "tetris.ms",
    "vt.ms",
    "tsv.ms",
    "acey-deucey.ms",
];

fn corpus() -> Vec<(&'static str, String)> {
    let dir = Path::new(env!("BENCH_CORPUS"));

    CORPUS
        .iter()
        .map(|name| {
            let source = std::fs::read_to_string(dir.join(name)).expect("corpus file");

            (*name, source)
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let files = corpus();
    let total: usize = files.iter().map(|(_, source)| source.len()).sum();

    let mut group = c.benchmark_group("corpus");

    group.throughput(Throughput::Bytes(total as u64));
    group.bench_function("lex", |b| {
        b.iter(|| {
            for (_, source) in &files {
                let lexer = Lexer::new(source, Dialect::MiniScript, Version::Latest);

                for token in lexer.tokens() {
                    black_box(token.expect("lexes"));
                }
            }
        });
    });
    group.bench_function("parse", |b| {
        b.iter(|| {
            for (_, source) in &files {
                let ast = Parser::new(source, ParserOptions::default()).parse();

                black_box(ast.expect("parses"));
            }
        });
    });
    group.bench_function("parse_unsafe", |b| {
        b.iter(|| {
            for (_, source) in &files {
                let output = Parser::new(source, ParserOptions::default()).parse_unsafe();

                black_box(output);
            }
        });
    });
    group.finish();

    let mut group = c.benchmark_group("parse");

    for (name, source) in &files {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), source, |b, source| {
            b.iter(|| {
                let ast = Parser::new(source, ParserOptions::default()).parse();

                black_box(ast.expect("parses"));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
