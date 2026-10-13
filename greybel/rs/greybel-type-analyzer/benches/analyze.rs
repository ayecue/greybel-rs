//! Analysis benchmarks; run with `cargo bench -p greybel-type-analyzer`.

use std::fmt::Write;
use std::hint::black_box;
use std::path::Path;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use greybel_core::{ColumnSettings, LineIndex};
use greybel_type_analyzer::{Analyzer, Database, Dialect, Workspace};

fn uri(path: &str) -> greybel_type_analyzer::Uri {
    greybel_type_analyzer::Uri::memory(path)
}

const CORPUS: &[&str] = &[
    "superstartrek.ms",
    "textAdventure.ms",
    "therapist.ms",
    "tetris.ms",
    "vt.ms",
    "tsv.ms",
    "acey-deucey.ms",
];

/// Byte interval between IDE query positions in each benchmark file.
const QUERY_STRIDE: usize = 97;

fn corpus() -> Vec<(String, String)> {
    let dir = Path::new(env!("BENCH_CORPUS"));

    CORPUS
        .iter()
        .map(|name| {
            let source = std::fs::read_to_string(dir.join(name)).expect("corpus file");

            (format!("/{name}"), source)
        })
        .collect()
}

fn loaded(dialect: Dialect, files: &[(String, String)]) -> Workspace<Database> {
    let mut workspace =
        Workspace::with_database(Database::default(), Analyzer::document_options(dialect));

    for (path, source) in files {
        workspace.set_file(&uri(path), source.as_str());
    }

    workspace
}

/// Generates `count` inheritance-chained custom types with properties, declared functions, a generic box, and
/// generic calls.
fn annotated(count: usize) -> String {
    let mut source = String::from(
        "// @type Box<T>\n// @property {T} value\n// @property {list<T>} items\nBox = {}\n\n\
         // @template T\n// @param {T} value\n// @returns {T}\nidentity = function(value)\n  return value\nend function\n\n",
    );

    for index in 0..count {
        let _ = writeln!(source, "// @vtype Type{index}");

        if index > 0 {
            let _ = writeln!(source, "// @extends Type{}", index - 1);
        }

        let _ = write!(
            source,
            "// @property {{number}} count{index}\n// @property {{Box<string>}} box{index}\n\
             // @function make{index}\n//   @param {{string}} name\n//   @returns {{Type{index}}}\n\n\
             // @define {{Type{index}}}\nvalue{index} = {{}}\n\
             label{index} = identity(value{index}.box{index}.value)\n\
             made{index} = value{index}.make{index}(\"x\").count0\n\n"
        );
    }

    source
}

/// Generates `count` list and map mutation chains using API mutators, index assignment, and `@mutates`, followed by
/// reads.
fn mutating(count: usize) -> String {
    let mut source = String::new();

    for index in 0..count {
        let _ = write!(
            source,
            "list{index} = []\nlist{index}.push({index})\nlist{index}.insert(0, \"s{index}\")\n\
             list{index}[1] = {{}}\nmap{index} = {{}}\nmap{index}.push(\"k{index}\")\n\
             // @param {{list<T>}} out\n// @param {{T}} item\n// @mutates out\n\
             fill{index} = function(out, item)\n  out.push(item)\nend function\n\
             fill{index}(list{index}, map{index}.indexes)\ntop{index} = list{index}.pop\n\n"
        );
    }

    source
}

/// Generates unannotated functions invoked with varied argument and receiver types.
fn calling(count: usize) -> String {
    let mut source = String::new();

    for index in 0..count {
        let _ = write!(
            source,
            "identity{index} = function(x)\n  return x\nend function\n\
             Base{index} = {{}}\nBase{index}.Create = function\n  return new self\nend function\n\
             Base{index}.label = function\n  return self.name\nend function\n\
             Child{index} = new Base{index}\nChild{index}.name = \"c{index}\"\n\
             make{index} = function(value)\n  return function()\n    return {{\"value\": value}}\n  end function\nend function\n\
             a{index} = identity{index}({index})\nb{index} = identity{index}(\"s{index}\")\n\
             c{index} = Child{index}.Create\nd{index} = c{index}.label\n\
             f{index} = make{index}(a{index})\ne{index} = f{index}.value\n\n"
        );
    }

    source
}

fn bench_corpus(c: &mut Criterion) {
    let files = corpus();
    let total: usize = files.iter().map(|(_, source)| source.len()).sum();
    let mut group = c.benchmark_group("corpus");

    group.throughput(Throughput::Bytes(total as u64));
    group.bench_function("analyze", |b| {
        b.iter_batched(
            || loaded(Dialect::MiniScript, &files),
            |workspace| {
                for (path, _) in &files {
                    black_box(Analyzer::new(&workspace).analysis(&uri(path)));
                }

                workspace
            },
            BatchSize::LargeInput,
        );
    });
    group.bench_function("diagnostics", |b| {
        b.iter_batched(
            || loaded(Dialect::MiniScript, &files),
            |workspace| {
                for (path, _) in &files {
                    black_box(Analyzer::new(&workspace).diagnostics(&uri(path)));
                }

                workspace
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();

    let mut group = c.benchmark_group("incremental");
    let mut workspace = loaded(Dialect::MiniScript, &files);

    for (path, _) in &files {
        black_box(Analyzer::new(&workspace).analysis(&uri(path)));
    }

    group.bench_function("unchanged", |b| {
        b.iter(|| {
            for (path, _) in &files {
                black_box(Analyzer::new(&workspace).analysis(&uri(path)));
            }
        });
    });

    let (path, source) = &files[1];
    let edited = format!("{source}\nedited = 1\n");
    let mut toggle = false;

    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("edit", |b| {
        b.iter(|| {
            toggle = !toggle;
            workspace.set_file(&uri(path), if toggle { edited.as_str() } else { source });
            black_box(Analyzer::new(&workspace).analysis(&uri(path)));
        });
    });
    group.finish();
}

fn bench_queries(c: &mut Criterion) {
    let files = corpus();
    let (path, source) = &files[1];
    let workspace = loaded(Dialect::MiniScript, &files[1..2]);

    Analyzer::new(&workspace)
        .analysis(&uri(path))
        .expect("analyzed");

    let lines = LineIndex::new(source);
    let positions: Vec<_> = (0..source.len())
        .step_by(QUERY_STRIDE)
        .filter(|&offset| source.is_char_boundary(offset))
        .map(|offset| lines.position(offset as u32, ColumnSettings::default()))
        .collect();
    let mut group = c.benchmark_group("queries");

    group.throughput(Throughput::Elements(positions.len() as u64));
    group.bench_function("hover", |b| {
        b.iter(|| {
            positions
                .iter()
                .for_each(|&at| drop(black_box(Analyzer::new(&workspace).hover(&uri(path), at))))
        });
    });
    group.bench_function("completions", |b| {
        b.iter(|| {
            positions.iter().for_each(|&at| {
                drop(black_box(
                    Analyzer::new(&workspace).completions(&uri(path), at),
                ))
            })
        });
    });
    group.bench_function("signature_help", |b| {
        b.iter(|| {
            positions.iter().for_each(|&at| {
                drop(black_box(
                    Analyzer::new(&workspace).signature_help(&uri(path), at),
                ))
            });
        });
    });
    group.bench_function("definition", |b| {
        b.iter(|| {
            positions.iter().for_each(|&at| {
                drop(black_box(
                    Analyzer::new(&workspace).definition(&uri(path), at),
                ))
            })
        });
    });
    group.bench_function("references", |b| {
        b.iter(|| {
            positions.iter().for_each(|&at| {
                drop(black_box(
                    Analyzer::new(&workspace).references(&uri(path), at),
                ))
            })
        });
    });
    group.bench_function("document_symbols", |b| {
        b.iter(|| black_box(Analyzer::new(&workspace).document_symbols(&uri(path))));
    });
    group.finish();
}

fn bench_annotations(c: &mut Criterion) {
    let source = annotated(200);
    let mut group = c.benchmark_group("annotations");

    group.throughput(Throughput::Bytes(source.len() as u64));

    let check = loaded(
        Dialect::MiniScript,
        &[("/main.src".to_owned(), source.clone())],
    );
    let analysis = Analyzer::new(&check)
        .analysis(&uri("/main.src"))
        .expect("analyzed");

    assert!(
        Analyzer::new(&check)
            .diagnostics(&uri("/main.src"))
            .is_empty(),
        "the generated program is clean"
    );
    assert_eq!(
        analysis
            .display(analysis.global_type("label199").expect("assigned"))
            .to_string(),
        "string"
    );
    group.bench_function("custom_types", |b| {
        b.iter_batched(
            || {
                loaded(
                    Dialect::MiniScript,
                    &[("/main.src".to_owned(), source.clone())],
                )
            },
            |workspace| {
                black_box(Analyzer::new(&workspace).analysis(&uri("/main.src")));

                workspace
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_mutations(c: &mut Criterion) {
    let source = mutating(200);
    let mut group = c.benchmark_group("mutations");

    group.throughput(Throughput::Bytes(source.len() as u64));

    let check = loaded(
        Dialect::MiniScript,
        &[("/main.src".to_owned(), source.clone())],
    );
    let analysis = Analyzer::new(&check)
        .analysis(&uri("/main.src"))
        .expect("analyzed");

    assert!(
        Analyzer::new(&check)
            .diagnostics(&uri("/main.src"))
            .is_empty(),
        "the generated program is clean"
    );
    assert_eq!(
        analysis
            .display(analysis.global_type("list199").expect("assigned"))
            .to_string(),
        "list<list<string> or map or number or string>"
    );
    group.bench_function("containers", |b| {
        b.iter_batched(
            || {
                loaded(
                    Dialect::MiniScript,
                    &[("/main.src".to_owned(), source.clone())],
                )
            },
            |workspace| {
                black_box(Analyzer::new(&workspace).analysis(&uri("/main.src")));

                workspace
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn bench_calls(c: &mut Criterion) {
    let source = calling(200);
    let mut group = c.benchmark_group("calls");

    group.throughput(Throughput::Bytes(source.len() as u64));

    let check = loaded(
        Dialect::MiniScript,
        &[("/main.src".to_owned(), source.clone())],
    );
    let analysis = Analyzer::new(&check)
        .analysis(&uri("/main.src"))
        .expect("analyzed");

    assert!(
        Analyzer::new(&check)
            .diagnostics(&uri("/main.src"))
            .is_empty(),
        "the generated program is clean"
    );

    let shown = |name: &str| {
        analysis
            .display(analysis.global_type(name).expect("assigned"))
            .to_string()
    };

    assert_eq!(
        ["a199", "b199", "d199", "e199"].map(shown),
        ["number", "string", "string", "number"]
    );
    group.bench_function("contexts", |b| {
        b.iter_batched(
            || {
                loaded(
                    Dialect::MiniScript,
                    &[("/main.src".to_owned(), source.clone())],
                )
            },
            |workspace| {
                black_box(Analyzer::new(&workspace).analysis(&uri("/main.src")));

                workspace
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_corpus,
    bench_queries,
    bench_annotations,
    bench_mutations,
    bench_calls
);

criterion_main!(benches);
