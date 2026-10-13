//! Integration tests for `greybel-lsp` over real pipes against an on-disk project.

use serde_json::json;

use crate::support::{
    Session, codes, initialized_session, session_with_main_open, shut_down_session,
};

#[test]
fn should_initialize_as_greybel_lsp() {
    let mut session = Session::start(env!("CARGO_BIN_EXE_greybel-lsp"));
    let initialized = session.initialize();

    assert_eq!(initialized["result"]["serverInfo"]["name"], "greybel-lsp");
}

#[test]
fn should_accept_the_stdio_flag_language_clients_pass() {
    let mut session = Session::start_with(env!("CARGO_BIN_EXE_greybel-lsp"), &["--stdio"]);
    let initialized = session.initialize();

    assert_eq!(initialized["result"]["serverInfo"]["name"], "greybel-lsp");
}

#[test]
fn should_report_diagnostics_of_a_project_on_disk() {
    let mut session = initialized_session(env!("CARGO_BIN_EXE_greybel-lsp"));
    let published = session.open_main();

    assert_eq!(
        codes(&published),
        ["unresolved-reference"],
        "greet comes from disk, nope from nowhere"
    );
}

#[test]
fn should_find_workspace_symbols_on_disk() {
    let mut session = session_with_main_open(env!("CARGO_BIN_EXE_greybel-lsp"));
    let symbols = session.workspace_symbols("greet");

    assert_eq!(
        symbols["result"][0]["location"]["uri"],
        session.project.uri("lib/greet.src"),
        "the workspace was found on disk"
    );
}

#[test]
fn should_leave_out_the_folders_excluded_by_default() {
    let mut session = session_with_main_open(env!("CARGO_BIN_EXE_greybel-lsp"));
    let vendored = session.workspace_symbols("vendored");
    let hidden = session.workspace_symbols("hidden");

    assert_eq!(vendored["result"], json!([]), "node_modules is excluded");
    assert_eq!(hidden["result"], json!([]), "dot folders are excluded");
}

#[test]
fn should_find_the_files_of_folders_no_longer_excluded() {
    let mut session = session_with_main_open(env!("CARGO_BIN_EXE_greybel-lsp"));

    session.configure(json!({ "typeAnalyzer": { "exclude": "**/.config/**" } }));

    let vendored = session.workspace_symbols("vendored");
    let hidden = session.workspace_symbols("hidden");

    assert_eq!(
        vendored["result"][0]["location"]["uri"],
        session.project.uri("node_modules/dep/vendored.src")
    );
    assert_eq!(hidden["result"], json!([]));
}

#[test]
fn should_read_the_files_itself() {
    let mut session = session_with_main_open(env!("CARGO_BIN_EXE_greybel-lsp"));

    session.workspace_symbols("greet");

    let asked: Vec<&String> = session
        .methods
        .iter()
        .filter(|method| method.starts_with("custom/"))
        .collect();

    assert!(asked.is_empty(), "the client was asked: {asked:?}");
}

#[test]
fn should_exit_with_zero_after_shutdown() {
    let mut session = shut_down_session(env!("CARGO_BIN_EXE_greybel-lsp"));
    let status = session.exit();

    assert_eq!(status.code(), Some(0));
}
