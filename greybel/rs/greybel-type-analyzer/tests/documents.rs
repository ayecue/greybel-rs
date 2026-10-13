//! Tests cross-file analysis for `#include`, `#import`, and `import_code`.

use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect};

use crate::support::documents::{
    dependency, dependency_edited, dependent_edited, load, missing_imports, module_import,
    shared_include,
};
use crate::support::workspace::{kinds, message_texts};
use crate::support::{globals_in, uri};

#[test]
fn should_share_the_global_scope_with_an_include() {
    let workspace = shared_include();

    let types = globals_in(&workspace, "/main.src", &["out", "count"]);

    assert_eq!(types, ["string", "number"]);
}

#[test]
fn should_resolve_the_globals_of_an_include() {
    let workspace = shared_include();

    let diagnostics = Analyzer::new(&workspace).diagnostics(&uri("/main.src"));

    assert!(diagnostics.is_empty());
}

#[test]
fn should_bind_the_module_exports_of_an_import() {
    let workspace = module_import();

    let types = globals_in(&workspace, "/main.src", &["name", "result"]);

    assert_eq!(types, ["string", "number"]);
}

#[test]
fn should_hide_what_an_imported_module_does_not_export() {
    let workspace = module_import();

    let main = message_texts(&workspace, "/main.src");

    assert_eq!(main, ["Cannot find \"secret\"."]);
}

#[test]
fn should_report_nothing_in_an_imported_module() {
    let workspace = module_import();

    let lib = Analyzer::new(&workspace).diagnostics(&uri("/lib.src"));

    assert!(lib.is_empty());
}

#[test]
fn should_run_import_code_in_place() {
    let workspace = load(
        Dialect::GreyScript,
        &[
            ("/main.src", "import_code(\"/lib.src\")\nn = count"),
            ("/lib.src", "count = 1"),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["n"]);

    assert_eq!(types, ["number"]);
}

#[test]
fn should_end_include_cycles() {
    let workspace = load(
        Dialect::Greybel,
        &[
            ("/main.src", "#include \"a.src\"\nx = a"),
            ("/a.src", "#include \"main.src\"\na = 1"),
        ],
    );

    let main = globals_in(&workspace, "/main.src", &["x"]);
    let a = globals_in(&workspace, "/a.src", &["a"]);

    assert_eq!(main, ["number"]);
    assert_eq!(a, ["number"]);
}

#[test]
fn should_type_from_a_dependency() {
    let workspace = dependency();

    let types = globals_in(&workspace, "/main.src", &["out"]);

    assert_eq!(types, ["number"]);
}

#[test]
fn should_carry_an_edit_of_a_dependency_to_its_dependents() {
    let workspace = dependency_edited();

    let types = globals_in(&workspace, "/main.src", &["out"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_keep_an_edited_dependency_after_editing_the_dependent() {
    let workspace = dependent_edited();

    let types = globals_in(&workspace, "/main.src", &["out"]);

    assert_eq!(types, ["list<string>"]);
}

#[test]
fn should_bind_any_to_a_missing_import() {
    let workspace = missing_imports();

    let types = globals_in(&workspace, "/main.src", &["HelloWord", "HelloName"]);

    assert_eq!(types, ["any", "any"]);
}

#[test]
fn should_report_missing_imports() {
    let workspace = missing_imports();

    let kinds = kinds(&workspace, "/main.src");

    assert_eq!(
        kinds,
        [DiagnosticKind::MissingImport, DiagnosticKind::MissingImport]
    );
}

#[test]
fn should_merge_imports_of_two_files_into_one_name() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#import tools from \"numbers.src\"\n#import tools from \"strings.src\"\nanswer = tools.answer",
            ),
            ("/numbers.src", "module.exports = {\"answer\": 123}"),
            ("/strings.src", "module.exports = {\"answer\": \"hi\"}"),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["answer"]);

    assert_eq!(types, ["number or string"]);
}

#[test]
fn should_define_globals_of_a_longer_included_file_in_time() {
    let library: String = (0..40)
        .map(|index| format!("v{index} = {index}\n"))
        .collect();
    let workspace = load(
        Dialect::Greybel,
        &[
            ("/main.src", "#include \"lib.src\"\nprint v39"),
            ("/lib.src", &library),
        ],
    );

    let diagnostics = Analyzer::new(&workspace).diagnostics(&uri("/main.src"));

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}
