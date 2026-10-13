//! Text renderers for parse-result snapshots.

use std::fmt::Write;

use greybel_core::{ColumnSettings, LineIndex, ParseError, Parser, ParserOptions};

fn render_errors(out: &mut String, title: &str, errors: &[ParseError], lines: &LineIndex) {
    if errors.is_empty() {
        return;
    }

    writeln!(out, "--- {title}").unwrap();
    for error in errors {
        let range = error.range(lines, ColumnSettings::default());

        writeln!(
            out,
            "{range} {:?}({}): {}",
            error.code(),
            error.code().as_u16(),
            error.message()
        )
        .unwrap();
        if let Some(hint) = error.hint() {
            writeln!(out, "    hint: {hint}").unwrap();
        }
    }
}

/// Renders a strict parse as `ok` and its tree, or as error diagnostics.
#[must_use]
pub fn render_strict(source: &str, options: &ParserOptions) -> String {
    match Parser::new(source, options.clone()).parse() {
        Ok(ast) => format!("ok\n{}\n", ast.dump(ast.root(), ColumnSettings::default())),
        Err(error) => {
            let mut out = String::new();

            render_errors(&mut out, "error", &[error], &LineIndex::new(source));

            out
        }
    }
}

/// Renders the strict result, unsafe tree, parser errors, and lexer errors.
#[must_use]
pub fn render(source: &str, options: &ParserOptions) -> String {
    let mut out = String::new();

    match Parser::new(source, options.clone()).parse() {
        Ok(_) => out.push_str("--- strict: ok\n"),
        Err(error) => render_errors(&mut out, "strict", &[error], &LineIndex::new(source)),
    }

    let output = Parser::new(source, options.clone()).parse_unsafe();
    let ast = &output.ast;

    writeln!(
        out,
        "--- tree\n{}",
        ast.dump(ast.root(), ColumnSettings::default())
    )
    .unwrap();
    render_errors(&mut out, "errors", &output.errors, ast.lines());
    render_errors(&mut out, "lexer errors", &output.lexer_errors, ast.lines());

    out
}

/// Parses `source` in strict and unsafe modes, asserting no errors and identical trees, and returns the tree dump.
#[must_use]
pub fn render_valid(source: &str, options: &ParserOptions) -> String {
    let lines = LineIndex::new(source);
    let at =
        |error: &ParseError| format!("{} {error}", error.range(&lines, ColumnSettings::default()));
    let ast = Parser::new(source, options.clone())
        .parse()
        .unwrap_or_else(|error| panic!("{}", at(&error)));
    let unsafe_output = Parser::new(source, options.clone()).parse_unsafe();

    if let Some(error) = unsafe_output.all_errors().next() {
        panic!("unsafe: {}", at(error));
    }

    let dump = ast.dump(ast.root(), ColumnSettings::default());

    assert_eq!(
        dump,
        unsafe_output
            .ast
            .dump(unsafe_output.ast.root(), ColumnSettings::default()),
        "trees differ"
    );

    dump
}

/// Renders `source` with [`render`] and asserts that strict parsing fails.
#[must_use]
pub fn render_invalid(source: &str, options: &ParserOptions) -> String {
    let rendered = render(source, options);

    assert!(
        !rendered.starts_with("--- strict: ok"),
        "parses without error"
    );

    rendered
}
