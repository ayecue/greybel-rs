//! Serializable lexer and parser output for snapshot comparisons.

use std::collections::BTreeMap;

use greybel_core::{
    Ast, AstNode, ColumnSettings, ErrorKind, Grammar, Lexer, LineIndex, NodeId, NodeKind,
    ParseError, Parser, ParserOptions, PositionEncoding, Span,
};
use serde_json::{Value, json};

/// Fixture-specific parser setup and extra AST dumps.
pub struct Setup {
    pub name: &'static str,
    /// Builds parser options from the fixture's dialect, version, file name, and environment.
    pub options: fn(&Value) -> ParserOptions,
    /// Returns dialect-specific AST collections (`imports`, and others) dumped with the fixture's column settings.
    pub extras: fn(&Ast, ColumnSettings) -> BTreeMap<String, Vec<String>>,
}

/// Returns UTF-16 column settings using the fixture's `tabWidth`, matching the TypeScript implementation.
pub fn columns(case: &Value) -> ColumnSettings {
    ColumnSettings::new(
        PositionEncoding::Utf16,
        u32::try_from(case["tabWidth"].as_u64().unwrap_or(1))
            .expect("value fits the target integer type"),
    )
}

fn error_json(error: &ParseError, lines: &LineIndex, columns: ColumnSettings) -> Value {
    let r = error.range(lines, columns);

    json!({
        "message": error.message(),
        "code": error.code().as_u16(),
        "range": [[r.start.line, r.start.character], [r.end.line, r.end.character]],
        "hint": error.hint(),
    })
}

fn errors_json(errors: &[ParseError], lines: &LineIndex, columns: ColumnSettings) -> Value {
    Value::Array(
        errors
            .iter()
            .map(|error| error_json(error, lines, columns))
            .collect(),
    )
}

pub fn lexer(
    content: &str,
    grammar: &'static Grammar,
    columns: ColumnSettings,
    unsafe_mode: bool,
) -> Value {
    let lexer = Lexer::with_grammar(content, grammar);
    let lines = lexer.lines();
    let mut walk = if unsafe_mode {
        lexer.tokens_unsafe()
    } else {
        lexer.tokens()
    };
    let mut tokens = Vec::new();
    let mut result = serde_json::Map::new();

    for token in walk.by_ref() {
        match token {
            Ok(token) => tokens.push(Value::String(
                token.display(content, lines, columns).to_string(),
            )),
            Err(error) => {
                result.insert("error".into(), error_json(&error, lines, columns));
            }
        }
    }

    result.insert("tokens".into(), Value::Array(tokens));
    result.insert(
        "lexerErrors".into(),
        errors_json(walk.stream().errors(), lines, columns),
    );

    Value::Object(result)
}

fn namespace_kinds(ast: &Ast) -> Value {
    let mut counts = BTreeMap::<&str, u64>::new();

    for &id in ast.list(ast.scope().namespaces) {
        let id = match ast.kind(id) {
            NodeKind::Member(member) => member.identifier,
            _ => id,
        };

        if let NodeKind::Identifier(identifier) = ast.kind(id) {
            *counts.entry(identifier.kind.as_str()).or_default() += 1;
        }
    }

    json!(counts)
}

fn comments_json(ast: &Ast, columns: ColumnSettings) -> Value {
    let texts = |spans: &[Span]| {
        spans
            .iter()
            .map(|&span| ast.slice(span))
            .collect::<Vec<_>>()
    };
    let mut out: Vec<(u32, u32, &str, Value)> = Vec::new();
    let mut stack = vec![ast.root()];
    let mut seen = std::collections::HashSet::<NodeId>::new();

    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }

        let node = ast.node(id);

        if let Some(comments) = ast.node_comments(id) {
            let ty = node.kind.type_name();
            let start = ast.lines().position(node.span.start, columns);

            out.push((
                start.line,
                start.character,
                ty,
                json!({
                    "type": ty,
                    "line": start.line,
                    "character": start.character,
                    "leading": texts(&comments.lead_comments),
                    "trailing": texts(&comments.trailing_comments),
                    "endTrailing": texts(&comments.end_trailing_comments),
                }),
            ));
        }

        stack.extend(ast.children(id));
    }

    out.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));

    Value::Array(out.into_iter().map(|e| e.3).collect())
}

pub fn parser(case: &Value, dialect: &Setup) -> Value {
    let columns = columns(case);
    let options = ParserOptions {
        preserve: case["preserve"].as_bool().unwrap_or(false),
        strict_mode: case["strictMode"].as_bool().unwrap_or(false),

        mono_number_parsing: false,
        ..(dialect.options)(case)
    };
    let preserve = options.preserve;
    let content = case["content"].as_str().expect("content");
    let lexer = Lexer::with_grammar(content, options.grammar());
    let lines = lexer.lines().clone();
    let parser = Parser::from_lexer(&lexer, options);
    let mut result = serde_json::Map::new();

    let (ast, errors, lexer_errors) = if case["unsafe"].as_bool().unwrap_or(false) {
        let output = parser.parse_unsafe();

        (Some(output.ast), output.errors, output.lexer_errors)
    } else {
        match parser.parse() {
            Ok(ast) => (Some(ast), Vec::new(), Vec::new()),
            Err(error) => {
                result.insert("error".into(), error_json(&error, &lines, columns));

                let lexer_error = error.kind == ErrorKind::Lexer;
                let list = vec![error];

                if lexer_error {
                    (None, Vec::new(), list)
                } else {
                    (None, list, Vec::new())
                }
            }
        }
    };

    if let Some(ast) = ast {
        result.insert("dump".into(), Value::String(ast.dump(ast.root(), columns)));
        result.insert("namespaces".into(), namespace_kinds(&ast));
        result.insert("literals".into(), json!(ast.literals.len()));
        result.insert("definitions".into(), json!(ast.scope().definitions.len()));
        result.insert("extras".into(), json!((dialect.extras)(&ast, columns)));
        if preserve {
            result.insert("comments".into(), comments_json(&ast, columns));
        }
    }

    result.insert("errors".into(), errors_json(&errors, &lines, columns));
    result.insert(
        "lexerErrors".into(),
        errors_json(&lexer_errors, &lines, columns),
    );

    Value::Object(result)
}
