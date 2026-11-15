//! Tests multi-entry workspaces, shared libraries, edits, and cross-file IDE features.

use greybel_core::Position;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_test_support::future::block_on;
use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect};

use crate::support::features::completion_labels;
use crate::support::workspace::{
    Executions, IMPORT_PREFIX, INCLUDE_PREFIX, LOG, LOG_SOURCE, MAIN, MAIN_SOURCE, MATH, TOOL,
    TOOL_SOURCE, UTIL, UTIL_SOURCE, analyze_all, analyzed_once, at, barrels, created_later,
    describe_places, edited, exports_across_folders, extra_included, extra_removed,
    file_completions, import_code_above_the_project, include_actions, included_by_a_root,
    included_in_a_loop, kinds, kinds_and_messages, labels, load, loaded, log_restored,
    message_texts, missing_later, opened_later, project, reference_places, renamed_files,
    run_by_several_entries, sorted, typed_in_tool, without_log, workspace, workspace_edits,
    workspace_symbols,
};
use crate::support::{globals_in, new_workspace, uri};

#[test]
fn should_type_every_entry_through_its_whole_chain() {
    let workspace = workspace();

    let main = globals_in(&workspace, MAIN, &["total", "label", "level"]);
    let tool = globals_in(&workspace, TOOL, &["name", "level"]);
    let util = globals_in(&workspace, UTIL, &["describe", "level"]);
    let math = Analyzer::new(&workspace).analysis(&uri(MATH));
    let log = Analyzer::new(&workspace).analysis(&uri(LOG));

    assert_eq!(main, ["number", "string", "number"]);
    assert_eq!(tool, ["string", "number"]);
    assert_eq!(util, ["function(value: number): string", "number"]);
    assert!(math.is_some());
    assert!(log.is_some());
}

#[test]
fn should_report_nothing_in_any_file_of_the_workspace() {
    let workspace = workspace();

    let reports: Vec<_> = [MAIN, TOOL, UTIL, MATH, LOG]
        .into_iter()
        .map(|path| (path, kinds_and_messages(&workspace, path)))
        .collect();

    for (path, messages) in reports {
        assert_eq!(messages, [], "{path}");
    }
}

#[test]
fn should_load_asynchronously_like_synchronously() {
    let provider = project();
    let mut asynchronous = new_workspace(Dialect::Greybel);
    let sync = workspace();
    let cases = [
        (MAIN, &["total", "label", "items", "names", "picked"][..]),
        (TOOL, &["name", "level"][..]),
    ];

    for entry in [MAIN, TOOL] {
        block_on(asynchronous.load_async(&uri(entry), &provider)).expect("loads");
    }

    let outcomes: Vec<_> = cases
        .iter()
        .map(|(path, names)| {
            (
                globals_in(&asynchronous, path, names),
                globals_in(&sync, path, names),
            )
        })
        .collect();

    for (asynchronous, synchronous) in outcomes {
        assert_eq!(asynchronous, synchronous);
    }
}

#[test]
fn should_define_a_library_reached_twice_once() {
    let workspace = workspace();

    let locations =
        Analyzer::new(&workspace).definition(&uri(MAIN), at(MAIN_SOURCE, "describe", 0));

    let places: Vec<(&str, Position)> = locations
        .iter()
        .map(|location| (location.uri.path(), location.range.start))
        .collect();

    assert_eq!(places, [(UTIL, at(UTIL_SOURCE, "describe", 0))]);
}

#[test]
fn should_keep_diagnostics_with_their_document() {
    let provider = MemoryProvider::from_iter([
        (MAIN, "#include \"lib/util.src\"\nx = helper\n"),
        (UTIL, "helper = 1\nprint undefinedName\n"),
    ]);
    let workspace = load(Dialect::Greybel, &provider, &[MAIN]);

    let main = kinds_and_messages(&workspace, MAIN);
    let util = kinds_and_messages(&workspace, UTIL);

    assert_eq!(main, []);
    assert_eq!(
        util,
        [(
            DiagnosticKind::UnresolvedReference,
            "Cannot find \"undefinedName\".".to_owned()
        )]
    );
}

#[test]
fn should_find_every_reference_from_the_definition() {
    let workspace = workspace();
    let expected = describe_places();

    let places = reference_places(&workspace, UTIL, at(UTIL_SOURCE, "describe", 0));

    assert_eq!(places, expected, "references from {UTIL}");
}

#[test]
fn should_rename_every_use_from_the_definition() {
    let workspace = workspace();

    let edited = renamed_files(&workspace, UTIL, at(UTIL_SOURCE, "describe", 0), "format");

    assert_eq!(edited, sorted(vec![MAIN, TOOL, UTIL]), "rename from {UTIL}");
}

#[test]
fn should_rename_a_name_defined_two_includes_away() {
    let workspace = workspace();

    let edited = renamed_files(&workspace, LOG, at(LOG_SOURCE, "note", 0), "trace");

    assert_eq!(edited, sorted(vec![LOG, UTIL, MAIN]));
}

#[test]
fn should_find_every_reference_from_a_use() {
    let workspace = workspace();
    let expected = describe_places();

    let from_main = reference_places(&workspace, MAIN, at(MAIN_SOURCE, "describe", 0));
    let from_tool = reference_places(&workspace, TOOL, at(TOOL_SOURCE, "describe", 0));

    assert_eq!(from_main, expected, "references from {MAIN}");
    assert_eq!(from_tool, expected, "references from {TOOL}");
}

#[test]
fn should_rename_every_use_from_a_use() {
    let workspace = workspace();

    let from_main = renamed_files(&workspace, MAIN, at(MAIN_SOURCE, "describe", 0), "format");
    let from_tool = renamed_files(&workspace, TOOL, at(TOOL_SOURCE, "describe", 0), "format");

    assert_eq!(
        from_main,
        sorted(vec![MAIN, TOOL, UTIL]),
        "rename from {MAIN}"
    );
    assert_eq!(
        from_tool,
        sorted(vec![MAIN, TOOL, UTIL]),
        "rename from {TOOL}"
    );
}

#[test]
fn should_hover_a_function_of_another_file() {
    let workspace = workspace();

    let hover = Analyzer::new(&workspace).hover(&uri(MAIN), at(MAIN_SOURCE, "describe", 0));

    let hover = hover.expect("a hover");

    assert!(hover.text.contains("value: number"), "{}", hover.text);

    let documentation = hover.documentation.expect("documented");

    assert_eq!(
        documentation.description.as_deref(),
        Some("Formats a value.")
    );
}

#[test]
fn should_help_with_the_signature_of_another_files_function() {
    let workspace = workspace();
    let call = at(MAIN_SOURCE, "describe(", 0);
    let inside = Position::new(
        call.line,
        call.character
            + u32::try_from("describe(".len()).expect("value fits the target integer type"),
    );

    let help = Analyzer::new(&workspace).signature_help(&uri(MAIN), inside);

    let help = help.expect("signature help");

    assert!(help.label.contains("value: number"), "{}", help.label);
    assert_eq!(help.active_parameter, 0);
}

#[test]
fn should_complete_globals_of_other_files() {
    let workspace = workspace();
    let line_end = at(MAIN_SOURCE, "\nnote(", 0);

    let labels = completion_labels(&workspace, MAIN, line_end);

    for name in ["describe", "add", "level", "note", "items"] {
        assert!(labels.iter().any(|label| label == name), "{name} completes");
    }
}

#[test]
fn should_flow_types_through_shared_globals() {
    let workspace = workspace();

    let main = globals_in(&workspace, MAIN, &["items", "names", "picked"]);
    let util = globals_in(&workspace, UTIL, &["items"]);

    assert_eq!(main, ["list<number>", "list<string>", "string"]);
    assert_eq!(util, ["list<unknown>"]);
}

#[test]
fn should_see_globals_of_other_files_when_shadowing() {
    let provider = MemoryProvider::from_iter([
        (
            MAIN,
            "#include \"lib/util.src\"\nf = function\n  helper = 2\n  return helper\nend function\n",
        ),
        (UTIL, "helper = 1\n"),
    ]);
    let workspace = load(Dialect::Greybel, &provider, &[MAIN]);

    let kinds = kinds(&workspace, MAIN);

    assert_eq!(kinds, [DiagnosticKind::Shadow]);
}

#[test]
fn should_remove_a_library() {
    let mut workspace = workspace();

    let removed = workspace.remove_file(&uri(LOG));

    assert!(removed);
}

#[test]
fn should_report_what_a_removed_library_provided() {
    let workspace = without_log();

    let util = kinds_and_messages(&workspace, UTIL);
    let main = kinds_and_messages(&workspace, MAIN);

    assert_eq!(
        util,
        [
            (
                DiagnosticKind::MissingImport,
                "Missing import: ../shared/log.src".to_owned()
            ),
            (
                DiagnosticKind::UnresolvedReference,
                "Cannot find \"note\".".to_owned()
            ),
            (
                DiagnosticKind::UnresolvedReference,
                "Cannot find \"level\".".to_owned()
            ),
        ]
    );
    assert!(main.contains(&(
        DiagnosticKind::UnresolvedReference,
        "Cannot find \"note\".".to_owned()
    )));
}

#[test]
fn should_clear_the_reports_when_a_removed_library_returns() {
    let workspace = log_restored();

    let reports: Vec<_> = [MAIN, TOOL, UTIL]
        .into_iter()
        .map(|path| (path, kinds_and_messages(&workspace, path)))
        .collect();

    for (path, messages) in reports {
        assert_eq!(messages, [], "{path}");
    }
}

#[test]
fn should_type_what_a_returned_library_provides() {
    let workspace = log_restored();

    let main = globals_in(&workspace, MAIN, &["level"]);

    assert_eq!(main, ["number"]);
}

#[test]
fn should_report_an_include_of_a_missing_file() {
    let (without_extension, _) = missing_later("later");
    let (with_extension, _) = missing_later("later.src");

    let without_extension = kinds(&without_extension, MAIN);
    let with_extension = kinds(&with_extension, MAIN);

    for kinds in [without_extension, with_extension] {
        assert_eq!(
            kinds,
            [
                DiagnosticKind::MissingImport,
                DiagnosticKind::UnresolvedReference
            ]
        );
    }
}

#[test]
fn should_clear_the_reports_when_a_missing_file_is_opened() {
    let workspace = opened_later("later");

    let messages = kinds_and_messages(&workspace, MAIN);

    assert_eq!(messages, []);
}

#[test]
fn should_type_from_a_missing_file_opened_later() {
    let workspace = opened_later("later");

    let main = globals_in(&workspace, MAIN, &["x"]);

    assert_eq!(main, ["number"]);
}

#[test]
fn should_clear_the_reports_of_a_file_with_extension_opened_later() {
    let workspace = opened_later("later.src");

    let messages = kinds_and_messages(&workspace, MAIN);

    assert_eq!(messages, []);
}

#[test]
fn should_type_from_a_file_with_extension_opened_later() {
    let workspace = opened_later("later.src");

    let main = globals_in(&workspace, MAIN, &["x"]);

    assert_eq!(main, ["number"]);
}

#[test]
fn should_clear_the_reports_of_a_missing_file_created_on_disk() {
    let workspace = created_later();

    let messages = kinds_and_messages(&workspace, MAIN);

    assert_eq!(messages, []);
}

#[test]
fn should_read_a_missing_file_created_on_disk_on_reload() {
    let workspace = created_later();

    let main = globals_in(&workspace, MAIN, &["x"]);

    assert_eq!(main, ["string"]);
}

#[test]
fn should_type_from_an_include_added_to_an_open_file() {
    let workspace = extra_included();

    let main = globals_in(&workspace, MAIN, &["got"]);

    assert_eq!(main, ["string"]);
}

#[test]
fn should_resolve_names_of_an_include_added_to_an_open_file() {
    let workspace = extra_included();

    let messages = kinds_and_messages(&workspace, MAIN);

    assert_eq!(messages, []);
}

#[test]
fn should_report_names_of_an_include_removed_from_an_open_file() {
    let workspace = extra_removed();

    let messages = kinds_and_messages(&workspace, MAIN);

    assert_eq!(
        messages,
        [(
            DiagnosticKind::UnresolvedReference,
            "Cannot find \"bonus\".".to_owned()
        )]
    );
}

#[test]
fn should_bind_the_exports_of_an_import_across_folders() {
    let workspace = exports_across_folders();

    let main = globals_in(&workspace, MAIN, &["x"]);

    assert_eq!(main, ["number"]);
}

#[test]
fn should_resolve_an_import_across_folders() {
    let workspace = exports_across_folders();

    let messages = kinds_and_messages(&workspace, MAIN);

    assert_eq!(messages, []);
}

#[test]
fn should_analyze_every_file_on_the_first_query() {
    let (mut workspace, executions) = Executions::workspace();

    loaded(&mut workspace, &project());

    let analyzed = analyze_all(&workspace);
    let ran = executions.take(&workspace);

    assert!(analyzed);
    assert_eq!(
        ran.files("analysis"),
        sorted(vec![MAIN, TOOL, UTIL, MATH, LOG])
    );
}

#[test]
fn should_rerun_nothing_without_an_edit() {
    let (workspace, executions) = analyzed_once();

    let analyzed = analyze_all(&workspace);
    let ran = executions.take(&workspace);

    assert!(analyzed);
    assert_eq!(ran.files("analysis"), [] as [&str; 0]);
}

#[test]
fn should_rerun_only_an_edited_entry() {
    let (mut workspace, executions) = edited(0);
    let [(path, source), ..] = workspace_edits();

    workspace.set_file(&uri(path), source.as_str());

    let analyzed = analyze_all(&workspace);
    let ran = executions.take(&workspace);

    assert!(analyzed);
    assert_eq!(ran.files("parse"), [TOOL]);
    assert_eq!(ran.files("analysis"), [TOOL]);
}

#[test]
fn should_rerun_the_entries_including_an_edited_library() {
    let (mut workspace, executions) = edited(1);
    let [_, (path, source), _] = workspace_edits();

    workspace.set_file(&uri(path), source.as_str());

    let analyzed = analyze_all(&workspace);
    let ran = executions.take(&workspace);

    assert!(analyzed);
    assert_eq!(ran.files("analysis"), sorted(vec![MAIN, MATH]));
}

#[test]
fn should_rerun_every_file_reaching_an_edited_shared_library() {
    let (mut workspace, executions) = edited(2);
    let [.., (path, source)] = workspace_edits();

    workspace.set_file(&uri(path), source.as_str());

    let analyzed = analyze_all(&workspace);
    let ran = executions.take(&workspace);

    assert!(analyzed);
    assert_eq!(
        ran.files("analysis"),
        sorted(vec![MAIN, TOOL, UTIL, MATH, LOG])
    );
}

#[test]
fn should_match_workspace_symbols_by_subsequence() {
    let workspace = workspace();

    let subsequence = workspace_symbols(&workspace, "DSC");
    let unmatched = workspace_symbols(&workspace, "message");

    assert_eq!(
        subsequence,
        [("describe".to_owned(), UTIL.to_owned(), None)]
    );
    assert_eq!(unmatched, Vec::<(String, String, Option<String>)>::new());
}

#[test]
fn should_list_every_workspace_symbol_for_an_empty_query() {
    let workspace = workspace();

    let names: Vec<String> = workspace_symbols(&workspace, "")
        .into_iter()
        .map(|(name, _, _)| name)
        .collect();

    for name in [
        "add", "append", "current", "describe", "level", "note", "total",
    ] {
        assert!(names.iter().any(|known| known == name), "{name} is listed");
    }
}

#[test]
fn should_name_the_function_of_a_nested_workspace_symbol() {
    let mut workspace = new_workspace(Dialect::Greybel);

    workspace.set_file(&uri(MAIN), "outer = function\n  inner = 1\nend function");

    let nested: Vec<(String, Option<String>)> = Analyzer::new(&workspace)
        .workspace_symbols("inner")
        .into_iter()
        .map(|symbol| (symbol.name, symbol.container))
        .collect();

    assert_eq!(nested, [("inner".to_owned(), Some("outer".to_owned()))]);
}

#[test]
fn should_complete_include_paths_relative_to_the_document() {
    let mut workspace = workspace();

    workspace.set_file(&uri(TOOL), INCLUDE_PREFIX);

    let completions = file_completions(&workspace, INCLUDE_PREFIX);

    for (label, _, new_text) in &completions {
        assert_eq!(new_text, label);
    }

    let paths: Vec<(String, String)> = completions
        .into_iter()
        .map(|(label, detail, _)| (label, detail))
        .collect();

    assert_eq!(
        paths,
        [
            ("lib/math.src".to_owned(), MATH.to_owned()),
            ("lib/util.src".to_owned(), UTIL.to_owned()),
        ]
    );
}

#[test]
fn should_complete_absolute_import_paths() {
    let mut workspace = workspace();

    typed_in_tool(&mut workspace, &[INCLUDE_PREFIX]);
    workspace.set_file(&uri(TOOL), IMPORT_PREFIX);

    let completions = file_completions(&workspace, IMPORT_PREFIX);

    for (label, _, new_text) in &completions {
        assert_eq!(new_text, label);
    }

    let paths: Vec<(String, String)> = completions
        .into_iter()
        .map(|(label, detail, _)| (label, detail))
        .collect();

    assert_eq!(paths, [(LOG.to_owned(), LOG.to_owned())]);
}

#[test]
fn should_complete_no_paths_in_plain_strings() {
    let mut workspace = workspace();

    typed_in_tool(&mut workspace, &[INCLUDE_PREFIX, IMPORT_PREFIX]);

    let source = "#include \"lib/util.src\"\nprint \"lib/";

    workspace.set_file(&uri(TOOL), source);

    let completions = file_completions(&workspace, source);

    assert!(completions.is_empty(), "plain strings complete no paths");
}

#[test]
fn should_complete_import_code_paths_relative_to_the_project() {
    let workspace = import_code_above_the_project();

    let paths = completion_labels(&workspace, TOOL, Position::new(2, 31));

    assert_eq!(paths, Vec::<String>::new(), "relative to /project, not /");
}

#[test]
fn should_replace_the_typed_prefix_of_an_import_code_path() {
    let mut workspace = import_code_above_the_project();
    let _ = Analyzer::new(&workspace).completions(&uri(TOOL), Position::new(2, 31));

    workspace.set_file(&uri(TOOL), "x = 1\nimport_code(\"lib/m");

    let item = Analyzer::new(&workspace)
        .completions(&uri(TOOL), Position::new(2, 19))
        .into_iter()
        .next();

    let item = item.expect("a path");

    assert_eq!(item.label, "lib/math.src");

    let edit = item.text_edit.expect("an edit");

    assert_eq!(
        (edit.range.start, edit.range.end),
        (Position::new(2, 14), Position::new(2, 19))
    );
}

#[test]
fn should_offer_to_include_the_file_defining_an_unresolved_name() {
    let (dialect, line) = (Dialect::Greybel, 3);

    let actions = include_actions(dialect, line);

    for (kind, ..) in &actions {
        assert_eq!(*kind, DiagnosticKind::UnresolvedReference);
    }

    let actions: Vec<_> = actions
        .into_iter()
        .map(|(_, title, preferred, text, start)| (title, preferred, text, start))
        .collect();

    assert_eq!(
        actions,
        [(
            "Add #include \"shared/log.src\";".to_owned(),
            true,
            "#include \"shared/log.src\";\n".to_owned(),
            Position::new(2, 1)
        )]
    );
}

#[test]
fn should_offer_include_and_import_code_in_greyscript() {
    let (dialect, line) = (Dialect::GreyScript, 3);

    let actions = include_actions(dialect, line);

    for (kind, ..) in &actions {
        assert_eq!(*kind, DiagnosticKind::UnresolvedReference);
    }

    assert_eq!(
        actions
            .into_iter()
            .map(|(_, title, preferred, _, _)| (title, preferred))
            .collect::<Vec<_>>(),
        [
            ("Add #include \"shared/log.src\";".to_owned(), true),
            ("Add import_code(\"shared/log.src\")".to_owned(), false),
        ]
    );
}

#[test]
fn should_offer_no_include_where_none_applies() {
    let undefined = (Dialect::Greybel, 4);
    let resolved = (Dialect::Greybel, 2);
    let miniscript = (Dialect::MiniScript, 3);

    let undefined = include_actions(undefined.0, undefined.1);
    let resolved = include_actions(resolved.0, resolved.1);
    let miniscript = include_actions(miniscript.0, miniscript.1);

    assert!(undefined.is_empty(), "nothing defines it");
    assert!(resolved.is_empty(), "add is resolved");
    assert!(miniscript.is_empty(), "MiniScript has no includes");
}

#[test]
fn should_run_a_file_included_by_a_root_from_the_root() {
    let workspace = included_by_a_root();

    let included = labels(&Analyzer::new(&workspace).entry_uris(&uri("/app/b.src")));
    let root = labels(&Analyzer::new(&workspace).entry_uris(&uri("/app/main.src")));

    assert_eq!(included, ["/app/main.src"]);
    assert_eq!(root, ["/app/main.src"]);
}

#[test]
fn should_check_a_file_included_by_a_root_where_it_runs() {
    let workspace = included_by_a_root();

    let included = message_texts(&workspace, "/app/b.src");
    let root = Analyzer::new(&workspace).diagnostics(&uri("/app/main.src"));

    assert_eq!(included, ["Cannot find \"missing\"."]);
    assert!(root.is_empty());
}

#[test]
fn should_hover_names_of_a_file_included_by_a_root() {
    let workspace = included_by_a_root();

    let result = Analyzer::new(&workspace).hover(&uri("/app/b.src"), Position::new(2, 7));
    let helper = Analyzer::new(&workspace).hover(&uri("/app/b.src"), Position::new(1, 11));

    assert_eq!(result.expect("a hover").text, "result: number");
    assert_eq!(
        helper
            .expect("a hover")
            .documentation
            .and_then(|documentation| documentation.description),
        Some("Doubles.".to_owned())
    );
}

#[test]
fn should_define_a_name_of_a_sibling_include() {
    let workspace = included_by_a_root();

    let definitions: Vec<String> = Analyzer::new(&workspace)
        .definition(&uri("/app/b.src"), Position::new(1, 11))
        .into_iter()
        .map(|location| location.uri.label())
        .collect();

    assert_eq!(definitions, ["/app/a.src"]);
}

#[test]
fn should_complete_a_name_of_a_sibling_include() {
    let workspace = included_by_a_root();

    let completions =
        Analyzer::new(&workspace).completions(&uri("/app/b.src"), Position::new(3, 7));

    assert!(completions.iter().any(|item| item.label == "helper"));
}

#[test]
fn should_run_a_file_from_every_entry_reaching_it() {
    let workspace = run_by_several_entries();

    let entries = labels(&Analyzer::new(&workspace).entry_uris(&uri("/app/b.src")));

    assert_eq!(entries, ["/app/main.src", "/app/other.src"]);
}

#[test]
fn should_report_what_any_entry_running_a_file_finds() {
    let workspace = run_by_several_entries();

    let messages = message_texts(&workspace, "/app/b.src");

    assert_eq!(
        messages,
        [
            "Cannot find \"helper\". (when run from /app/other.src)",
            "Unused parameter: unused",
        ],
        "an error of one entry names it; one found by every entry appears once, as is"
    );
}

#[test]
fn should_read_later_definitions_in_time_in_a_file_included_in_a_loop() {
    let workspace = included_in_a_loop();

    let step = Analyzer::new(&workspace).diagnostics(&uri("/app/step.src"));

    assert!(step.is_empty(), "{step:?}");
}

#[test]
fn should_report_a_read_before_definition_in_a_file_included_once() {
    let workspace = included_in_a_loop();

    let plain = kinds(&workspace, "/app/plain.src");

    assert_eq!(plain, [DiagnosticKind::UseBeforeDefinition]);
}

#[test]
fn should_run_the_files_of_barrels_from_the_entry() {
    let workspace = barrels();

    let entries = labels(&Analyzer::new(&workspace).entry_uris(&uri("/app/views/page.src")));

    assert_eq!(entries, ["/app/main.src"]);
}

#[test]
fn should_type_globals_of_files_another_barrel_includes() {
    let workspace = barrels();
    let hover = |line: u32, column: u32| {
        Analyzer::new(&workspace)
            .hover(&uri("/app/views/page.src"), Position::new(line, column))
            .expect("a hover")
            .text
    };

    let text = hover(2, 2);
    let total = hover(3, 2);

    assert_eq!(text, "text: string");
    assert_eq!(total, "total: number");
}

#[test]
fn should_resolve_names_of_files_another_barrel_includes() {
    let workspace = barrels();

    let page = message_texts(&workspace, "/app/views/page.src");
    let stray = message_texts(&workspace, "/app/stray.src");

    assert_eq!(page, ["Cannot find \"unknownThing\"."]);
    assert_eq!(stray, ["Cannot find \"User\"."]);
}
