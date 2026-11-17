//! Tests the code navigation tools and rename on a project folder.

use serde_json::json;

use crate::support::code::{LIB, path, project};
use crate::support::{Client, OPTIONS, run};

#[test]
fn should_list_the_symbols_of_a_file_as_a_tree() {
    let folder = project("symbols");

    run(async {
        let mut client = Client::connect(OPTIONS).await;
        let (symbols, failed) = client
            .call(
                "document_symbols",
                json!({ "path": path(&folder, "lib.src") }),
            )
            .await;

        assert!(!failed, "{symbols}");
        assert!(symbols.starts_with("variable App"), "{symbols}");
        assert!(symbols.contains("\n  property utils"), "{symbols}");
        assert!(
            symbols.contains("\n    function format: function(value: string): string  (lines 6-8)"),
            "{symbols}"
        );
    });
}

#[test]
fn should_find_symbols_by_part_of_their_name_path() {
    let folder = project("find");

    run(async {
        let mut client = Client::connect(OPTIONS).await;
        let (found, _) = client
            .call(
                "find_symbol",
                json!({ "query": "utils.form", "path": folder.display().to_string(), "include_code": true }),
            )
            .await;
        let (none, _) = client
            .call(
                "find_symbol",
                json!({ "query": "nothing", "path": folder.display().to_string() }),
            )
            .await;

        assert!(
            found.starts_with("function App.utils.format: function(value: string): string at "),
            "{found}"
        );
        assert!(found.contains("lib.src:6:"), "{found}");
        assert!(
            found.contains("\n  Wraps a value in brackets.\n"),
            "{found}"
        );
        assert!(found.contains("return \"[\" + value + \"]\""), "{found}");
        assert_eq!(none, "No symbol matches \"nothing\".");
    });
}

#[test]
fn should_find_references_describe_and_list_calls_by_symbol_name() {
    let folder = project("navigate");
    let main = path(&folder, "main.src");

    run(async {
        let mut client = Client::connect(OPTIONS).await;
        let target = json!({ "symbol": "App.utils.format", "path": folder.display().to_string() });
        let (references, _) = client.call("find_references", target.clone()).await;
        let (description, _) = client.call("describe", target.clone()).await;
        let (callers, _) = client
            .call(
                "call_hierarchy",
                json!({ "symbol": "format", "path": folder.display().to_string(), "direction": "callers" }),
            )
            .await;
        let (callees, _) = client
            .call(
                "call_hierarchy",
                json!({ "path": main, "line": 2, "column": 1, "direction": "callees" }),
            )
            .await;

        assert!(references.contains("main.src:3:"), "{references}");
        assert!(
            references.contains(": return App.utils.format(\"hi \" + name)"),
            "{references}"
        );
        assert!(
            description.contains("Wraps a value in brackets."),
            "{description}"
        );
        assert!(description.contains("Defined at:\n"), "{description}");
        assert!(
            callers.contains("Called by:\n  function greet at "),
            "{callers}"
        );
        assert!(
            callees.contains("Calls:\n  function App.utils.format at "),
            "{callees}"
        );
    });
}

#[test]
fn should_ask_for_the_full_name_of_an_ambiguous_symbol() {
    let folder = project("ambiguous");

    run(async {
        let mut client = Client::connect(OPTIONS).await;
        let (error, failed) = client
            .call(
                "describe",
                json!({ "symbol": "x", "path": folder.display().to_string() }),
            )
            .await;
        let (local, _) = client
            .call(
                "describe",
                json!({ "symbol": "second.x", "path": folder.display().to_string() }),
            )
            .await;

        assert!(failed, "{error}");
        assert!(error.contains("first.x at "), "{error}");
        assert!(error.contains("second.x at "), "{error}");
        assert!(local.contains("main.src:10:"), "{local}");
    });
}

#[test]
fn should_rename_a_symbol_in_every_file() {
    let folder = project("rename");

    run(async {
        let mut client = Client::connect(OPTIONS).await;
        let target = |dry_run: bool| {
            json!({
                "symbol": "App.utils.format",
                "path": folder.display().to_string(),
                "new_name": "wrap",
                "dry_run": dry_run
            })
        };
        let (preview, _) = client.call("rename", target(true)).await;
        let unchanged = std::fs::read_to_string(folder.join("lib.src")).expect("reads");
        let (renamed, failed) = client.call("rename", target(false)).await;
        let (invalid, invalid_failed) = client
            .call(
                "rename",
                json!({ "symbol": "greet", "path": folder.display().to_string(), "new_name": "not a name" }),
            )
            .await;

        assert!(
            preview.starts_with("Would rename 2 occurrences in 2 files:"),
            "{preview}"
        );
        assert_eq!(unchanged, LIB);
        assert!(!failed, "{renamed}");
        assert!(
            std::fs::read_to_string(folder.join("lib.src"))
                .expect("reads")
                .contains("App.utils.wrap = function(value)")
        );
        assert!(
            std::fs::read_to_string(folder.join("main.src"))
                .expect("reads")
                .contains("return App.utils.wrap(\"hi \" + name)")
        );
        assert!(invalid_failed, "{invalid}");
    });
}
