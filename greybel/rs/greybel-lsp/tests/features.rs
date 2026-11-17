//! Tests hovers, completions, navigation, renames, symbols, quick fixes, and formatting.

use greybel_lsp::types::{self as lsp, Request as _};
use serde_json::json;

use crate::support::{
    Client, LIBRARY, MAIN, inlay_hints, lsp_position, project, project_completing_greet,
    project_renaming_greet, project_with_definition_links, project_with_main_open, rename_greet,
    uri,
};

#[test]
fn should_hover_with_the_documentation_of_an_include() {
    let mut client = project();

    client.requests.clear();
    client.open("main.src", MAIN);

    let hover: Option<lsp::Hover> =
        client.request::<lsp::HoverRequest>(Client::position("main.src", 1, 11));
    let lsp::Contents::MarkupContent(content) = hover.expect("a hover").contents else {
        panic!("markup");
    };

    assert_eq!(content.kind, lsp::MarkupKind::Markdown);
    assert!(
        content
            .value
            .starts_with("```greyscript-signature\ngreet: function("),
        "{}",
        content.value
    );
    assert!(content.value.contains("Says hello."), "{}", content.value);
}

#[test]
fn should_defer_completion_documentation_to_resolve() {
    let mut client = project_completing_greet();

    let greet = client.completion("main.src", 1, 1, "greet");

    assert!(greet.documentation.is_none());
    assert!(greet.data.is_some());
}

#[test]
fn should_document_a_completion_on_resolve() {
    let mut client = project_completing_greet();
    let greet = client.completion("main.src", 1, 1, "greet");

    let resolved =
        client.request::<lsp::CompletionResolveRequest>(serde_json::to_value(greet).unwrap());

    let Some(lsp::Documentation::MarkupContent(documentation)) = resolved.documentation else {
        panic!("documentation");
    };

    assert!(
        documentation.value.contains("Says hello."),
        "{}",
        documentation.value
    );
}

#[test]
fn should_rename_in_closed_files_including_the_library() {
    let mut client = project_renaming_greet();
    let edit = client
        .request::<lsp::RenameRequest>(rename_greet("welcome"))
        .expect("an edit");
    let mut files: Vec<String> = edit
        .changes
        .expect("changes")
        .into_keys()
        .map(|uri| uri.0)
        .collect();

    files.sort();
    assert_eq!(
        files,
        [uri("lib/greet.src"), uri("main.src"), uri("tool.src")]
    );
}

#[test]
fn should_refuse_to_rename_to_an_invalid_name() {
    let mut client = project_renaming_greet();

    client.request::<lsp::RenameRequest>(rename_greet("welcome"));

    let invalid = client
        .request_value(
            lsp::RenameRequest::METHOD.as_str(),
            rename_greet("not valid"),
        )
        .expect_err("an invalid name");

    assert!(invalid.message.contains("not valid"), "{}", invalid.message);
}

#[test]
fn should_go_to_the_definition_in_an_include() {
    let mut client = project_with_main_open();

    let definition = client.request::<lsp::DefinitionRequest>(Client::position("main.src", 1, 11));

    let Some(lsp::DefinitionResponse::Definition(lsp::Definition::LocationList(locations))) =
        definition
    else {
        panic!("a definition");
    };

    assert_eq!(locations[0].uri.0, uri("lib/greet.src"));
    assert_eq!(locations[0].range.start, lsp_position(3, 0));
}

#[test]
fn should_link_the_complete_import_path_to_its_file() {
    let mut client = project_with_definition_links();

    let definition = client.request::<lsp::DefinitionRequest>(Client::position("main.src", 0, 14));

    let Some(lsp::DefinitionResponse::DefinitionLinkList(links)) = definition else {
        panic!("definition links");
    };
    let origin = links[0].origin_selection_range.expect("an origin");

    assert_eq!(links[0].target_uri.0, uri("lib/greet.src"));
    assert_eq!(
        (origin.start, origin.end),
        (lsp_position(0, 10), lsp_position(0, 23))
    );
}

#[test]
fn should_fold_comments_and_function_bodies() {
    let mut client = project_with_main_open();

    let folds = client
        .request::<lsp::FoldingRangeRequest>(
            json!({ "textDocument": { "uri": uri("lib/greet.src") } }),
        )
        .expect("folding ranges");
    let folds: Vec<_> = folds
        .iter()
        .map(|fold| (fold.start_line, fold.end_line, fold.kind.clone()))
        .collect();

    assert_eq!(
        folds,
        [(0, 2, Some(lsp::FoldingRangeKind::Comment)), (3, 4, None)]
    );
}

#[test]
fn should_select_from_a_name_out_to_its_statement() {
    let mut client = project_with_main_open();

    let selections = client
        .request::<lsp::SelectionRangeRequest>(json!({
            "textDocument": { "uri": uri("main.src") },
            "positions": [{ "line": 1, "character": 12 }]
        }))
        .expect("selection ranges");
    let mut ranges = Vec::new();
    let mut selection = Some(&selections[0]);

    while let Some(current) = selection {
        ranges.push((current.range.start, current.range.end));
        selection = current.parent.as_deref();
    }

    assert_eq!(ranges[0], (lsp_position(1, 10), lsp_position(1, 15)));
    assert_eq!(
        ranges[ranges.len() - 2],
        (lsp_position(1, 0), lsp_position(1, 22))
    );
}

#[test]
fn should_send_inlay_hints() {
    let mut client = project_with_main_open();

    let hints = inlay_hints(&mut client);

    assert_eq!(
        hints,
        [
            (lsp_position(1, 7), ": string".to_owned()),
            (lsp_position(1, 16), "name:".to_owned()),
            (lsp_position(2, 6), "s:".to_owned()),
        ]
    );
}

#[test]
fn should_leave_out_inlay_hints_the_settings_turn_off() {
    let settings = json!({ "typeAnalyzer": { "inlayHints": { "variableTypes": false } } });
    let mut client =
        Client::with_settings(&[("lib/greet.src", LIBRARY), ("main.src", MAIN)], settings);

    client.open("main.src", MAIN);

    assert_eq!(
        inlay_hints(&mut client),
        [
            (lsp_position(1, 16), "name:".to_owned()),
            (lsp_position(2, 6), "s:".to_owned()),
        ]
    );
}

#[test]
fn should_go_to_the_type_definition_of_a_map() {
    let source = "m = {\"a\": 1}\nprint m\n";
    let mut client = Client::started(&[("main.src", source)]);

    client.open("main.src", source);

    let definition =
        client.request::<lsp::TypeDefinitionRequest>(Client::position("main.src", 1, 6));
    let Some(lsp::TypeDefinitionResponse::Definition(lsp::Definition::LocationList(locations))) =
        definition
    else {
        panic!("a type definition");
    };

    assert_eq!(locations[0].range.start, lsp_position(0, 4));
}

#[test]
fn should_show_and_present_tag_colors() {
    let source = "print \"<color=#FF8000>a</color>\"\n";
    let mut client = Client::started(&[("main.src", source)]);

    client.open("main.src", source);

    let colors = client.request::<lsp::DocumentColorRequest>(
        json!({ "textDocument": { "uri": uri("main.src") } }),
    );
    let presentations = client.request::<lsp::ColorPresentationRequest>(json!({
        "textDocument": { "uri": uri("main.src") },
        "color": { "red": 0.0, "green": 1.0, "blue": 0.0, "alpha": 0.5 },
        "range": colors[0].range
    }));

    assert_eq!(colors.len(), 1);
    assert_eq!(colors[0].range.start, lsp_position(0, 14));
    assert_eq!(presentations[0].label, "#00FF0080");
}

#[test]
fn should_highlight_every_use_of_a_variable() {
    let mut client = project_with_main_open();
    let highlights = client
        .request::<lsp::DocumentHighlightRequest>(Client::position("main.src", 1, 1))
        .expect("highlights");

    assert_eq!(highlights.len(), 2, "message is assigned and printed");
}

#[test]
fn should_help_with_the_signature_of_an_included_function() {
    let mut client = project_with_main_open();
    let help = client
        .request::<lsp::SignatureHelpRequest>(Client::position("main.src", 1, 16))
        .expect("signature help");

    assert!(
        help.signatures[0].label.contains("name: string"),
        "{}",
        help.signatures[0].label
    );
}

#[test]
fn should_find_workspace_symbols_by_a_fuzzy_query() {
    let mut client = project_with_main_open();
    let symbols = client
        .request_value(
            lsp::WorkspaceSymbolRequest::METHOD.as_str(),
            json!({ "query": "grt" }),
        )
        .expect("symbols");
    let symbols: Vec<lsp::WorkspaceSymbol> =
        serde_json::from_value(symbols).expect("workspace symbols");
    let names: Vec<(&str, &str)> = symbols
        .iter()
        .map(|symbol| {
            let lsp::WorkspaceSymbolLocation::Location(location) = &symbol.location else {
                panic!("a location");
            };

            (
                symbol.base_symbol_information.name.as_str(),
                location.uri.0.as_str(),
            )
        })
        .collect();

    assert_eq!(names, [("greet", uri("lib/greet.src").as_str())]);
}

#[test]
fn should_offer_quick_fixes_including_the_defining_file() {
    let mut client = project();

    client.open("other.src", "print greet(\"x\")\n");

    let actions = client
        .request::<lsp::CodeActionRequest>(json!({
            "textDocument": { "uri": uri("other.src") },
            "range": { "start": { "line": 0, "character": 6 }, "end": { "line": 0, "character": 11 } },
            "context": { "diagnostics": [] }
        }))
        .expect("actions");
    let titles: Vec<String> = actions
        .into_iter()
        .map(|action| match action {
            lsp::CodeActionResponse::CodeAction(action) => action.title,
            lsp::CodeActionResponse::Command(command) => command.title,
        })
        .collect();

    assert_eq!(
        titles,
        [
            "Add #include \"lib/greet.src\";",
            "Add import_code(\"lib/greet.src\")",
            "Ignore \"unresolved-reference\" on this line",
            "Ignore \"unresolved-reference\" in this file"
        ]
    );
}

#[test]
fn should_format_with_the_beautify_settings() {
    let mut client = Client::with_settings(
        &[],
        json!({ "transpiler": { "beautify": { "indentation": "Whitespace", "indentationSpaces": 2 } } }),
    );

    client.open("main.src", "if true then\nprint 1\nend if");

    let edits = client
        .request::<lsp::DocumentFormattingRequest>(json!({
            "textDocument": { "uri": uri("main.src") },
            "options": { "tabSize": 2, "insertSpaces": true }
        }))
        .expect("edits");

    assert_eq!(edits.len(), 1);
    assert!(
        edits[0].new_text.contains("\n  print 1"),
        "{:?}",
        edits[0].new_text
    );
    assert_eq!(edits[0].range.end, lsp_position(2, 6));
}

#[test]
fn should_format_without_rewriting_greybel_syntax() {
    let mut client = Client::started(&[]);
    let source = "x = 1\nx += 2\ny = 6 | 3\nf = function(a = [])\n\treturn a\nend function\ndebugger\nprint #line\ns = \"a\nb\"\nn = 0xFF + 1_000";

    client.open("main.src", source);

    let edits = client
        .request::<lsp::DocumentFormattingRequest>(json!({
            "textDocument": { "uri": uri("main.src") },
            "options": { "tabSize": 2, "insertSpaces": true }
        }))
        .expect("edits");

    assert_eq!(edits, Vec::new(), "formatted text equals the source");
}

#[test]
fn should_not_format_a_document_with_syntax_errors() {
    let mut client = Client::started(&[]);

    client.open("main.src", "if true then\nprint 1\n");

    let edits = client.request::<lsp::DocumentFormattingRequest>(json!({
        "textDocument": { "uri": uri("main.src") },
        "options": { "tabSize": 2, "insertSpaces": true }
    }));

    assert_eq!(edits, None, "a partly parsed document isn't rewritten");
}
