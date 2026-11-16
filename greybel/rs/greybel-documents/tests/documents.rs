//! Tests incremental document loading, checking, and updates with an in-memory provider.

use greybel_core::Dialect;
use greybel_documents::{
    Cycle, DependencyResolver, DocumentOptions, ImportKind, ModuleGraph, NativeImportMode,
    Resolution, ResourceRequest, Workspace,
};
use greybel_test_support::documents::graph_outline;
use greybel_test_support::future::block_on;

use crate::support::{
    PROJECT, TYPE_INCLUDE, broken_main_in_unsafe_mode, content, graph, miniscript_main, options,
    recorded, relative, resolver_answered_a_of_an_edited_main, resolver_answered_after_a_was_set,
    resolver_answered_after_an_unrelated_change, resolver_answered_b, resolver_answered_c,
    resolver_reading_a_of_an_edited_main, resolver_reading_a_set_meanwhile,
    resolver_told_where_a_is, resolver_told_where_b_is, resolver_told_where_c_is, step, uri,
    workspace_after_an_edit_adding_an_include, workspace_after_an_edit_without_new_includes,
    workspace_including_a, workspace_reloaded_with_a_new_include,
    workspace_with_a_removed_dependency, workspace_with_missing_targets,
    workspace_with_missing_targets_checked, workspace_with_missing_targets_created,
    workspace_with_the_include_line_restored, workspace_with_two_edits,
    workspace_without_the_include_line,
};

#[test]
fn should_read_every_file_once() {
    let provider = recorded(PROJECT);

    let graph: ModuleGraph =
        ModuleGraph::load(&uri("/main.src"), &options(Dialect::Greybel), &provider).expect("loads");

    let mut reads: Vec<String> = provider
        .take()
        .into_iter()
        .filter(|call| call.starts_with("read"))
        .collect();

    reads.sort();
    assert_eq!(reads, ["read /a.src", "read /b.src", "read /main.src"]);
    assert_eq!(
        graph_outline(&graph),
        [
            "/main.src ModuleId(0) [ModuleId(1), ModuleId(2)]",
            "/b.src ModuleId(1) [ModuleId(2)]",
            "/a.src ModuleId(2) []"
        ]
    );

    let main = graph.module(graph.entry().expect("entry"));

    assert_eq!(main.injection(main.ast.injects[0]), Some("print 1"));

    let b = main.dependencies()[0].module;

    assert!(graph.module(b).is_imported());
    assert!(!graph.module(main.dependencies()[1].module).is_imported());
}

#[test]
fn should_load_the_same_graph_asynchronously() {
    let provider = recorded(PROJECT);
    let expected = graph_outline(&graph(PROJECT).expect("loads"));

    let streamed: ModuleGraph = block_on(ModuleGraph::load_async(
        &uri("/main.src"),
        &options(Dialect::Greybel),
        &provider,
    ))
    .expect("loads");

    assert_eq!(graph_outline(&streamed), expected);
}

#[test]
fn should_record_a_cycle_not_fail_on_it() {
    let files = [
        ("/main.src", "#import a from \"a.src\""),
        ("/a.src", "#import b from \"b.src\""),
        ("/b.src", "#import a from \"a.src\""),
    ];
    let graph = graph(&files).expect("loads");

    assert_eq!(
        graph.cycle(),
        Some(&Cycle {
            from: uri("/b.src"),
            to: uri("/a.src")
        })
    );
    assert_eq!(graph.len(), 3);
    assert_eq!(
        graph.dependency_order(graph.entry().expect("entry")).len(),
        3
    );
}

#[test]
fn should_name_who_asked_for_a_missing_file() {
    let projects: [&[(&str, &str)]; 3] = [
        &[("/main.src", "#include \"missing\"")],
        &[("/main.src", "print #inject \"missing\"")],
        &[],
    ];
    let [include, injection, target] = projects.map(|files| graph(files).err());

    assert_eq!(
        include.as_deref(),
        Some("Dependency /missing.src does not exist...")
    );
    assert_eq!(
        injection.as_deref(),
        Some("Injection /missing.src does not exist...")
    );
    assert_eq!(
        target.as_deref(),
        Some("Target /main.src does not exist...")
    );
}

#[test]
fn should_report_errors_level_by_level() {
    let projects: [&[(&str, &str)]; 2] = [
        &[
            ("/main.src", "#include \"a.src\"\n#include \"missing\""),
            ("/a.src", "print ("),
        ],
        &[
            ("/main.src", "#include \"a.src\"\n#include \"missing\""),
            ("/a.src", "#include \"broken.src\""),
            ("/broken.src", "print ("),
        ],
    ];
    let [syntax, missing] = projects.map(|files| graph(files).err().expect("broken"));

    assert_eq!(
        syntax,
        "got EOL where number, string, or identifier is required"
    );
    assert_eq!(missing, "Dependency /missing.src does not exist...");
}

#[test]
fn should_read_nothing_after_an_edit_without_new_includes() {
    let (mut workspace, provider) = workspace_after_an_edit_without_new_includes();

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    assert!(provider.take().is_empty(), "nothing new to read");
}

#[test]
fn should_read_only_a_new_include_after_an_edit() {
    let (mut workspace, provider) = workspace_after_an_edit_adding_an_include();

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    assert_eq!(provider.take(), ["relative /main.src b.src", "read /b.src"]);
}

#[test]
fn should_resolve_a_new_include_after_an_edit() {
    let (workspace, _provider) = workspace_reloaded_with_a_new_include();
    let main = workspace.file(&uri("/main.src")).expect("loaded");
    let db = workspace.db();
    let dependencies: Vec<String> = main
        .resolved_imports(db)
        .iter()
        .map(|(_, file)| file.uri(db).label())
        .collect();

    assert_eq!(dependencies, ["/a.src", "/b.src"]);
}

#[test]
fn should_keep_the_dependents_after_an_edit_adding_an_include() {
    let (workspace, _provider) = workspace_reloaded_with_a_new_include();
    let dependents = workspace.dependents(&uri("/a.src"));

    assert_eq!(dependents, [uri("/main.src")]);
}

#[test]
fn should_build_the_graph_with_a_new_include_after_an_edit() {
    let (workspace, _provider) = workspace_reloaded_with_a_new_include();
    let graph: ModuleGraph = workspace.module_graph(&uri("/main.src")).expect("loads");

    assert_eq!(graph.len(), 3);
}

#[test]
fn should_report_a_removed_file_as_missing() {
    let workspace = workspace_with_a_removed_dependency();
    let error = workspace
        .module_graph(&uri("/main.src"))
        .err()
        .map(|error| error.to_string());

    assert_eq!(
        error.as_deref(),
        Some("Dependency /a.src does not exist...")
    );
}

#[test]
fn should_build_the_graph_again_once_a_removed_file_is_set() {
    let mut workspace = workspace_with_a_removed_dependency();
    let _ = workspace.module_graph(&uri("/main.src"));

    workspace.set_file(&uri("/a.src"), "x = 2");

    let graph = workspace.module_graph(&uri("/main.src"));

    assert!(graph.is_ok());
}

#[test]
fn should_fail_the_graph_while_targets_are_missing() {
    let (workspace, _provider) = workspace_with_missing_targets();
    let graph = workspace.module_graph(&uri("/main.src"));

    assert!(graph.is_err());
}

#[test]
fn should_look_up_a_missing_target_on_every_load() {
    let (mut workspace, provider) = workspace_with_missing_targets_checked();

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    assert_eq!(
        provider.take(),
        [
            "relative /main.src a",
            "relative /main.src b.src",
            "read /a.src",
            "read /b.src"
        ]
    );
}

#[test]
fn should_build_the_graph_once_missing_targets_exist() {
    let (workspace, _provider) = workspace_with_missing_targets_created();
    let graph = workspace.module_graph(&uri("/main.src"));

    assert!(graph.is_ok());
}

#[test]
fn should_read_nothing_once_every_target_is_read() {
    let (mut workspace, provider) = workspace_with_missing_targets_created();
    let _ = workspace.module_graph(&uri("/main.src"));

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    assert!(provider.take().is_empty(), "everything is read");
}

#[test]
fn should_parse_a_tree_with_its_errors_in_unsafe_mode() {
    let (workspace, file) = broken_main_in_unsafe_mode();

    let parsed = file.parse(workspace.db());

    assert!(parsed.ast().is_some());
    assert!(!parsed.errors().is_empty());
}

#[test]
fn should_find_the_imports_of_a_broken_file_in_unsafe_mode() {
    let (workspace, file) = broken_main_in_unsafe_mode();
    let _ = file.parse(workspace.db());

    let imports = file.imports(workspace.db());

    assert_eq!(imports.dependencies().count(), 1);
}

#[test]
fn should_parse_miniscript_without_errors() {
    let (workspace, file) = miniscript_main();

    let parsed = file.parse(workspace.db());

    assert!(parsed.errors().is_empty());
}

#[test]
fn should_find_no_imports_in_miniscript() {
    let (workspace, file) = miniscript_main();
    let _ = file.parse(workspace.db());

    let imports = file.imports(workspace.db());

    assert!(imports.is_empty());
}

#[test]
fn should_follow_the_options_for_native_imports() {
    let files = [
        (
            "/main.src",
            "import_code(\"a.src\")\nimport_code(\"b.src\") // no-emit",
        ),
        ("/a.src", "x = 1"),
        ("/b.src", "y = 2"),
    ];
    let native = ImportKind::NativeImport;
    let [evaluated, emitted] =
        [NativeImportMode::Evaluation, NativeImportMode::Emission].map(|native_imports| {
            let options = DocumentOptions {
                native_imports,
                ..options(Dialect::GreyScript)
            };
            let graph: ModuleGraph =
                ModuleGraph::load(&uri("/main.src"), &options, &recorded(&files)).expect("loads");

            graph
                .module(graph.entry().expect("entry"))
                .dependencies()
                .iter()
                .map(|edge| (edge.kind, graph.module(edge.module).uri.label()))
                .collect::<Vec<_>>()
        });

    assert_eq!(
        evaluated,
        [(native, "/a.src".to_owned()), (native, "/b.src".to_owned())]
    );
    assert_eq!(emitted, [(native, "/a.src".to_owned())]);
}

#[test]
fn should_ask_where_an_include_is_first() {
    let mut workspace = Workspace::new(options(Dialect::Greybel));

    workspace.set_file(&uri("/main.src"), "#include \"a.src\"");

    let mut resolver = DependencyResolver::new(uri("/main.src"));
    let requests = step(&mut resolver, &workspace);

    assert_eq!(requests, [relative("/main.src", "a.src")]);
}

#[test]
fn should_ask_to_read_an_include_once_it_is_found() {
    let (workspace, mut resolver) = resolver_told_where_a_is();
    let requests = step(&mut resolver, &workspace);

    assert_eq!(requests, [ResourceRequest::Read(uri("/a.src"))]);
}

#[test]
fn should_resolve_while_the_workspace_changes_in_between() {
    let (workspace, mut resolver) = resolver_answered_after_an_unrelated_change();
    let resolution = resolver.next(&workspace);

    assert_eq!(resolution, Resolution::Complete);
}

#[test]
fn should_record_dependents_when_resolving_step_by_step() {
    let (workspace, mut resolver) = resolver_answered_after_an_unrelated_change();

    resolver.next(&workspace);

    let dependents = workspace.dependents(&uri("/a.src"));

    assert_eq!(dependents, [uri("/main.src")]);
}

#[test]
fn should_keep_the_set_text_over_a_read_answered_later() {
    let (mut workspace, mut resolver) = resolver_reading_a_set_meanwhile();

    resolver.process_responses(&mut workspace, vec![content("x = 1")]);

    let a = workspace.file(&uri("/a.src")).expect("set");

    assert_eq!(
        a.text(workspace.db()).as_deref(),
        Some("#include \"b.src\""),
        "the editor's text wins over the outdated read"
    );
}

#[test]
fn should_follow_the_includes_of_the_set_text() {
    let (workspace, mut resolver) = resolver_answered_after_a_was_set();
    let requests = step(&mut resolver, &workspace);

    assert_eq!(requests, [relative("/a.src", "b.src")]);
}

#[test]
fn should_ask_to_read_an_include_of_the_set_text() {
    let (workspace, mut resolver) = resolver_told_where_b_is();
    let requests = step(&mut resolver, &workspace);

    assert_eq!(requests, [ResourceRequest::Read(uri("/b.src"))]);
}

#[test]
fn should_resolve_the_includes_of_the_set_text() {
    let (workspace, mut resolver) = resolver_answered_b();
    let resolution = resolver.next(&workspace);

    assert_eq!(resolution, Resolution::Complete);
}

#[test]
fn should_record_dependents_of_the_set_text() {
    let (workspace, mut resolver) = resolver_answered_b();

    resolver.next(&workspace);

    let [of_a, of_b] = [uri("/a.src"), uri("/b.src")].map(|uri| workspace.dependents(&uri));

    assert_eq!(of_a, [uri("/main.src")]);
    assert_eq!(of_b, [uri("/a.src")]);
}

#[test]
fn should_drop_a_read_for_an_include_no_longer_walked() {
    let (mut workspace, mut resolver) = resolver_reading_a_of_an_edited_main();

    resolver.process_responses(&mut workspace, vec![content("x = 1")]);
    assert!(
        workspace.file(&uri("/a.src")).is_none(),
        "the answer for the old include is dropped"
    );
}

#[test]
fn should_restart_the_walk_when_a_walked_file_changes() {
    let (workspace, mut resolver) = resolver_answered_a_of_an_edited_main();
    let requests = step(&mut resolver, &workspace);

    assert_eq!(requests, [relative("/main.src", "c.src")]);
}

#[test]
fn should_ask_to_read_an_include_of_the_restarted_walk() {
    let (workspace, mut resolver) = resolver_told_where_c_is();
    let requests = step(&mut resolver, &workspace);

    assert_eq!(requests, [ResourceRequest::Read(uri("/c.src"))]);
}

#[test]
fn should_resolve_the_restarted_walk() {
    let (workspace, mut resolver) = resolver_answered_c();
    let resolution = resolver.next(&workspace);

    assert_eq!(resolution, Resolution::Complete);
}

#[test]
fn should_record_dependents_of_the_restarted_walk() {
    let (workspace, mut resolver) = resolver_answered_c();

    resolver.next(&workspace);

    let dependents = workspace.dependents(&uri("/c.src"));

    assert_eq!(dependents, [uri("/main.src")]);
}

#[test]
fn should_have_no_version_for_an_unknown_file() {
    let workspace = Workspace::new(options(Dialect::Greybel));
    let version = workspace.version(&uri("/main.src"));

    assert_eq!(version, None);
}

#[test]
fn should_start_the_version_at_zero() {
    let mut workspace = Workspace::new(options(Dialect::Greybel));

    workspace.set_file(&uri("/main.src"), "x = 1");

    let version = workspace.version(&uri("/main.src"));

    assert_eq!(version, Some(0));
}

#[test]
fn should_keep_the_version_when_the_text_is_the_same() {
    let mut workspace = Workspace::new(options(Dialect::Greybel));

    workspace.set_file(&uri("/main.src"), "x = 1");
    workspace.set_file(&uri("/main.src"), "x = 1");

    let version = workspace.version(&uri("/main.src"));

    assert_eq!(version, Some(0), "the same text is no change");
}

#[test]
fn should_count_up_the_version_on_a_text_change() {
    let workspace = workspace_with_two_edits();
    let version = workspace.version(&uri("/main.src"));

    assert_eq!(version, Some(1));
}

#[test]
fn should_report_the_removal_of_a_known_file() {
    let mut workspace = workspace_with_two_edits();
    let removed = workspace.remove_file(&uri("/main.src"));

    assert!(removed);
}

#[test]
fn should_count_a_removal_as_a_text_change() {
    let mut workspace = workspace_with_two_edits();

    workspace.remove_file(&uri("/main.src"));

    let version = workspace.version(&uri("/main.src"));

    assert_eq!(version, Some(2));
}

#[test]
fn should_report_no_removal_of_an_unknown_file() {
    let mut workspace = workspace_with_two_edits();

    workspace.remove_file(&uri("/main.src"));

    let removed = workspace.remove_file(&uri("/missing.src"));

    assert!(!removed);
}

#[test]
fn should_record_an_unknown_removed_file_as_missing() {
    let mut workspace = workspace_with_two_edits();

    workspace.remove_file(&uri("/missing.src"));

    let recorded = workspace.file(&uri("/missing.src")).is_some();
    let text = workspace.text(&uri("/missing.src"));

    assert_eq!((recorded, text), (true, None));
}

#[test]
fn should_record_the_includer_as_a_dependent() {
    let workspace = workspace_including_a();
    let dependents = workspace.dependents(&uri("/a.src"));

    assert_eq!(dependents, [uri("/main.src")]);
}

#[test]
fn should_end_the_dependency_when_the_include_line_is_removed() {
    let workspace = workspace_without_the_include_line();
    let dependents = workspace.dependents(&uri("/a.src"));

    assert!(dependents.is_empty());
}

#[test]
fn should_restore_the_dependency_with_the_include_line() {
    let workspace = workspace_with_the_include_line_restored();
    let dependents = workspace.dependents(&uri("/a.src"));

    assert_eq!(dependents, [uri("/main.src")]);
}

#[test]
fn should_end_the_dependencies_of_a_removed_file() {
    let mut workspace = workspace_with_the_include_line_restored();

    workspace.remove_file(&uri("/main.src"));

    let dependents = workspace.dependents(&uri("/a.src"));

    assert!(dependents.is_empty(), "a removed file depends on nothing");
}

#[test]
fn should_read_an_entry_not_in_the_workspace_first() {
    let provider = recorded(&[("/main.src", "#include \"a\""), ("/a.src", "x = 1")]);
    let mut workspace = Workspace::new(options(Dialect::Greybel));

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    assert_eq!(
        provider.take(),
        ["read /main.src", "relative /main.src a", "read /a.src"]
    );

    let main = workspace.file(&uri("/main.src")).expect("read");

    assert_eq!(main.uri(workspace.db()).to_string(), "memory:///main.src");
}

#[test]
fn should_read_a_type_include_without_adding_it_to_the_graph() {
    let provider = recorded(TYPE_INCLUDE);

    let graph: ModuleGraph =
        ModuleGraph::load(&uri("/main.src"), &options(Dialect::Greybel), &provider).expect("loads");

    let mut reads: Vec<String> = provider
        .take()
        .into_iter()
        .filter(|call| call.starts_with("read"))
        .collect();

    reads.sort();
    assert_eq!(reads, ["read /a.src", "read /main.src", "read /types.src"]);
    assert_eq!(
        graph_outline(&graph),
        [
            "/main.src ModuleId(0) [ModuleId(1)]",
            "/a.src ModuleId(1) []"
        ]
    );
}
