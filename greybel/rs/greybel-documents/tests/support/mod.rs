//! Shared fixtures and resolver drivers for `greybel-documents` integration tests.

#![allow(dead_code)]

use greybel_core::{Dialect, ParserOptions};
use greybel_documents::{
    DependencyResolver, DocumentOptions, ModuleGraph, Resolution, ResourceRequest,
    ResourceResponse, SourceFile, Uri, Workspace,
};
use greybel_documents_memory_provider::MemoryProvider;
use greybel_test_support::documents::CallRecorder;

pub fn uri(path: &str) -> Uri {
    MemoryProvider::uri(path)
}

pub fn options(dialect: Dialect) -> DocumentOptions {
    DocumentOptions {
        parser: ParserOptions::new(dialect),
        ..DocumentOptions::default()
    }
}

/// Creates an unsafe-mode workspace with malformed `/main.src` containing `#include "a.src"`, and returns the
/// workspace and source file.
pub fn broken_main_in_unsafe_mode() -> (Workspace, SourceFile) {
    let mut workspace = Workspace::new(DocumentOptions {
        unsafe_mode: true,
        ..options(Dialect::Greybel)
    });

    workspace.set_file(&uri("/main.src"), "print (\n#include \"a.src\"");

    let file = workspace.file(&uri("/main.src")).expect("set");

    (workspace, file)
}

/// Creates a MiniScript workspace with `/main.ms` containing `x = 1`, and returns the workspace and source file.
pub fn miniscript_main() -> (Workspace, SourceFile) {
    let mut workspace = Workspace::new(options(Dialect::MiniScript));

    workspace.set_file(&uri("/main.ms"), "x = 1");

    let file = workspace.file(&uri("/main.ms")).expect("set");

    (workspace, file)
}

/// Creates a `MemoryProvider` from `files` and records every resource request.
pub fn recorded(files: &[(&str, &str)]) -> CallRecorder<MemoryProvider> {
    CallRecorder::new(files.iter().copied().collect())
}

pub fn graph(files: &[(&str, &str)]) -> Result<ModuleGraph, String> {
    ModuleGraph::load(
        &uri("/main.src"),
        &options(Dialect::Greybel),
        &recorded(files),
    )
    .map_err(|error| error.to_string())
}

pub const PROJECT: &[(&str, &str)] = &[
    (
        "/main.src",
        "print #inject \"a.src\";\n#import b from \"b.src\"\n#include \"a.src\"",
    ),
    ("/a.src", "print 1"),
    ("/b.src", "#include \"a.src\""),
];

pub fn step(resolver: &mut DependencyResolver, workspace: &Workspace) -> Vec<ResourceRequest> {
    match resolver.next(workspace) {
        Resolution::Requests(requests) => requests,
        Resolution::Complete => Vec::new(),
    }
}

pub fn relative(source: &str, target: &str) -> ResourceRequest {
    ResourceRequest::RelativeTo {
        source: uri(source),
        target: target.to_owned(),
    }
}

pub fn content(text: &str) -> ResourceResponse {
    ResourceResponse::Content(Some(text.to_owned()))
}

/// Loads `/main.src` and its dependencies from `files`, returning the workspace and recording provider.
fn loaded(files: &[(&str, &str)]) -> (Workspace, CallRecorder<MemoryProvider>) {
    let provider = recorded(files);
    let mut workspace = Workspace::new(options(Dialect::Greybel));

    workspace.load(&uri("/main.src"), &provider).expect("loads");

    (workspace, provider)
}

/// Loads `/main.src` and `/a.src`, then edits `/main.src` without adding includes; `/b.src` remains unused.
pub fn workspace_after_an_edit_without_new_includes() -> (Workspace, CallRecorder<MemoryProvider>) {
    let (mut workspace, provider) = loaded(&[
        ("/main.src", "#include \"a.src\""),
        ("/a.src", "x = 1"),
        ("/b.src", "y = 2"),
    ]);

    provider.take();
    workspace.set_file(&uri("/main.src"), "print 1\n#include \"a.src\"");

    (workspace, provider)
}

/// Starts with [`workspace_after_an_edit_without_new_includes`] and adds `#include "b.src"` to `/main.src`.
pub fn workspace_after_an_edit_adding_an_include() -> (Workspace, CallRecorder<MemoryProvider>) {
    let (mut workspace, provider) = workspace_after_an_edit_without_new_includes();

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    provider.take();
    workspace.set_file(&uri("/main.src"), "#include \"a.src\"\n#include \"b.src\"");

    (workspace, provider)
}

/// Reloads `/main.src` after the new include and clears the provider's recorded requests.
pub fn workspace_reloaded_with_a_new_include() -> (Workspace, CallRecorder<MemoryProvider>) {
    let (mut workspace, provider) = workspace_after_an_edit_adding_an_include();

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    provider.take();

    (workspace, provider)
}

/// Starts with [`workspace_including_a`] and removes `/a.src`.
pub fn workspace_with_a_removed_dependency() -> Workspace {
    let mut workspace = workspace_including_a();

    workspace.remove_file(&uri("/a.src"));

    workspace
}

/// Loads `/main.src` with includes for `a` and `b.src` while neither target exists.
pub fn workspace_with_missing_targets() -> (Workspace, CallRecorder<MemoryProvider>) {
    loaded(&[("/main.src", "#include \"a\"\n#include \"b.src\"")])
}

/// Computes the missing-target workspace's module graph and clears the provider's recorded requests.
pub fn workspace_with_missing_targets_checked() -> (Workspace, CallRecorder<MemoryProvider>) {
    let (workspace, provider) = workspace_with_missing_targets();
    let _ = workspace.module_graph(&uri("/main.src"));

    provider.take();

    (workspace, provider)
}

/// Creates the missing `/a.src` and `/b.src` files and reloads `/main.src`.
pub fn workspace_with_missing_targets_created() -> (Workspace, CallRecorder<MemoryProvider>) {
    let (mut workspace, mut provider) = workspace_with_missing_targets_checked();

    workspace.load(&uri("/main.src"), &provider).expect("loads");
    provider.take();
    provider.inner.insert("/a.src", "x = 1");
    provider.inner.insert("/b.src", "y = 2");
    workspace.load(&uri("/main.src"), &provider).expect("loads");
    provider.take();

    (workspace, provider)
}

/// Sets `/main.src` to include `a.src` and resolves that include to URI `/a.src`.
pub fn resolver_told_where_a_is() -> (Workspace, DependencyResolver) {
    let mut workspace = Workspace::new(options(Dialect::Greybel));

    workspace.set_file(&uri("/main.src"), "#include \"a.src\"");

    let mut resolver = DependencyResolver::new(uri("/main.src"));

    step(&mut resolver, &workspace);
    resolver.process_responses(&mut workspace, vec![ResourceResponse::Uri(uri("/a.src"))]);

    (workspace, resolver)
}

/// Advances the resolver from [`resolver_told_where_a_is`] until it requests content for `/a.src`.
pub fn resolver_reading_a() -> (Workspace, DependencyResolver) {
    let (workspace, mut resolver) = resolver_told_where_a_is();

    step(&mut resolver, &workspace);

    (workspace, resolver)
}

/// Sets an unrelated file before answering the pending `/a.src` content request.
pub fn resolver_answered_after_an_unrelated_change() -> (Workspace, DependencyResolver) {
    let (mut workspace, mut resolver) = resolver_reading_a();

    workspace.set_file(&uri("/other.src"), "z = 3");
    resolver.process_responses(&mut workspace, vec![content("x = 1")]);

    (workspace, resolver)
}

/// Changes `/a.src` to include `b.src` while its original content request is pending.
pub fn resolver_reading_a_set_meanwhile() -> (Workspace, DependencyResolver) {
    let (mut workspace, resolver) = resolver_reading_a();

    workspace.set_file(&uri("/a.src"), "#include \"b.src\"");

    (workspace, resolver)
}

/// Answers the outdated `/a.src` content request after `/a.src` was edited.
pub fn resolver_answered_after_a_was_set() -> (Workspace, DependencyResolver) {
    let (mut workspace, mut resolver) = resolver_reading_a_set_meanwhile();

    resolver.process_responses(&mut workspace, vec![content("x = 1")]);

    (workspace, resolver)
}

/// Resolves the newly included `b.src` to URI `/b.src` after answering the outdated `/a.src` read.
pub fn resolver_told_where_b_is() -> (Workspace, DependencyResolver) {
    let (mut workspace, mut resolver) = resolver_answered_after_a_was_set();

    step(&mut resolver, &workspace);
    resolver.process_responses(&mut workspace, vec![ResourceResponse::Uri(uri("/b.src"))]);

    (workspace, resolver)
}

/// Supplies the contents of `/b.src` after its URI has been resolved.
pub fn resolver_answered_b() -> (Workspace, DependencyResolver) {
    let (mut workspace, mut resolver) = resolver_told_where_b_is();

    step(&mut resolver, &workspace);
    resolver.process_responses(&mut workspace, vec![content("y = 2")]);

    (workspace, resolver)
}

/// Changes `/main.src` to include `c.src` while the original `/a.src` content request is pending.
pub fn resolver_reading_a_of_an_edited_main() -> (Workspace, DependencyResolver) {
    let (mut workspace, resolver) = resolver_reading_a();

    workspace.set_file(&uri("/main.src"), "#include \"c.src\"");

    (workspace, resolver)
}

/// Answers the original `/a.src` content request after `/main.src` was edited.
pub fn resolver_answered_a_of_an_edited_main() -> (Workspace, DependencyResolver) {
    let (mut workspace, mut resolver) = resolver_reading_a_of_an_edited_main();

    resolver.process_responses(&mut workspace, vec![content("x = 1")]);

    (workspace, resolver)
}

/// Resolves the newly included `c.src` to URI `/c.src` after answering the original `/a.src` read.
pub fn resolver_told_where_c_is() -> (Workspace, DependencyResolver) {
    let (mut workspace, mut resolver) = resolver_answered_a_of_an_edited_main();

    step(&mut resolver, &workspace);
    resolver.process_responses(&mut workspace, vec![ResourceResponse::Uri(uri("/c.src"))]);

    (workspace, resolver)
}

/// Supplies the contents of `/c.src` after its URI has been resolved.
pub fn resolver_answered_c() -> (Workspace, DependencyResolver) {
    let (mut workspace, mut resolver) = resolver_told_where_c_is();

    step(&mut resolver, &workspace);
    resolver.process_responses(&mut workspace, vec![content("z = 3")]);

    (workspace, resolver)
}

/// Sets `/main.src` to `x = 1`, sets the same text again, then changes it to `x = 2`.
pub fn workspace_with_two_edits() -> Workspace {
    let mut workspace = Workspace::new(options(Dialect::Greybel));
    let main = uri("/main.src");

    workspace.set_file(&main, "x = 1");
    workspace.set_file(&main, "x = 1");
    workspace.set_file(&main, "x = 2");

    workspace
}

/// Loads `/main.src` with `#include "/a.src"` and `/a.src` containing `x = 1`.
pub fn workspace_including_a() -> Workspace {
    loaded(&[("/main.src", "#include \"a.src\""), ("/a.src", "x = 1")]).0
}

/// Starts with [`workspace_including_a`] and removes its include line.
pub fn workspace_without_the_include_line() -> Workspace {
    let mut workspace = workspace_including_a();

    workspace.set_file(&uri("/main.src"), "x = 2");

    workspace
}

/// Starts with [`workspace_without_the_include_line`] and restores its include line.
pub fn workspace_with_the_include_line_restored() -> Workspace {
    let mut workspace = workspace_without_the_include_line();

    workspace.set_file(&uri("/main.src"), "#include \"a.src\"");

    workspace
}
