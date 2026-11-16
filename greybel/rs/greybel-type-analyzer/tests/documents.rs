//! Tests cross-file analysis for `#include`, `#import`, and `import_code`.

use std::fmt::Write as _;

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
fn should_share_the_custom_object_with_imported_code() {
    let workspace = load(
        Dialect::GreyScript,
        &[
            (
                "/main.src",
                "get_custom_object.count = 1\nimport_code(\"/lib.src\")\nname = get_custom_object.name",
            ),
            (
                "/lib.src",
                "get_custom_object.name = \"n\"\ncount = get_custom_object.count",
            ),
        ],
    );

    let main = globals_in(&workspace, "/main.src", &["name", "count"]);

    assert_eq!(main, ["string", "number"]);
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
fn should_read_the_module_of_the_last_import_of_a_name() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#import tools from \"numbers.src\"\nfirst = tools.answer\n#import tools from \"strings.src\"\nanswer = tools.answer",
            ),
            ("/numbers.src", "module.exports = {\"answer\": 123}"),
            ("/strings.src", "module.exports = {\"answer\": \"hi\"}"),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["first", "answer"]);

    assert_eq!(types, ["number", "string"]);
}

#[test]
fn should_define_globals_of_a_longer_included_file_in_time() {
    let library = (0..40).fold(String::new(), |mut library, index| {
        let _ = writeln!(library, "v{index} = {index}");

        library
    });
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

#[test]
fn should_report_reads_before_the_include_that_assigns_them() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "print helper\n#include \"util.src\"\nprint helper",
            ),
            ("/util.src", "helper = 1"),
        ],
    );

    let messages = message_texts(&workspace, "/main.src");

    assert_eq!(messages, ["\"helper\" is used before it is assigned."]);
}

#[test]
fn should_report_reads_before_the_import_that_binds_them() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "print tools\n#import tools from \"lib.src\"\nprint tools",
            ),
            ("/lib.src", "module.exports = {}"),
        ],
    );

    let messages = message_texts(&workspace, "/main.src");

    assert_eq!(messages, ["\"tools\" is used before it is assigned."]);
}

#[test]
fn should_run_an_include_again_at_each_inclusion() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#include \"counter.src\"\ncount = \"x\"\nbetween = count\n#include \"counter.src\"\nagain = count",
            ),
            ("/counter.src", "count = 1"),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["between", "again"]);

    assert_eq!(types, ["string", "number or string"]);
}

#[test]
fn should_see_what_a_module_assigns_outside_it_after_the_import() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "flag = 1\nbefore = flag\n#import lib from \"lib.src\"\nafter = flag",
            ),
            ("/lib.src", "globals.flag = \"set\"\nmodule.exports = {}"),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["before", "after"]);

    assert_eq!(types, ["number", "number or string"]);
}

#[test]
fn should_infer_a_module_without_the_facts_of_the_importing_code() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "value = \"s\"\nvalue = 1\n#import lib from \"lib.src\"\ncopy = lib.copy",
            ),
            ("/lib.src", "module.exports = {\"copy\": value}"),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["copy"]);

    assert_eq!(types, ["number or string"]);
}

#[test]
fn should_report_import_code_after_code_in_a_file_reached_through_import_code() {
    let workspace = load(
        Dialect::GreyScript,
        &[
            (
                "/main.src",
                "print 0\nimport_code(\"/head.src\")\nimport_code(\"/late.src\")",
            ),
            ("/head.src", "import_code(\"/a.src\")\nprint 1"),
            ("/late.src", "print 2\nimport_code(\"/b.src\")"),
            ("/a.src", "a = 1"),
            ("/b.src", "b = 1"),
        ],
    );

    let main = kinds(&workspace, "/main.src");
    let head = kinds(&workspace, "/head.src");
    let late = kinds(&workspace, "/late.src");

    assert_eq!(main, []);
    assert_eq!(head, []);
    assert_eq!(late, [DiagnosticKind::MisplacedImportCode]);
}

#[test]
fn should_take_only_the_type_declarations_of_a_type_include() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#include type \"types.src\"\n// @define {Point}\npoint = {}\nx = point.x\nprint helper",
            ),
            (
                "/types.src",
                "// @vtype Point\n// @property {number} x\nhelper = 1",
            ),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["x"]);
    let messages = message_texts(&workspace, "/main.src");
    let entries: Vec<String> = Analyzer::new(&workspace)
        .entry_uris(&uri("/types.src"))
        .iter()
        .map(|entry| entry.path().to_owned())
        .collect();

    assert_eq!(types, ["number"]);
    assert_eq!(messages, ["Cannot find \"helper\"."]);
    assert_eq!(entries, ["/types.src"]);
}

#[test]
fn should_report_a_missing_type_include() {
    let workspace = load(
        Dialect::Greybel,
        &[("/main.src", "#include type \"types.src\"")],
    );

    let kinds = kinds(&workspace, "/main.src");

    assert_eq!(kinds, [DiagnosticKind::MissingImport]);
}

#[test]
fn should_bind_the_names_of_an_import_pattern() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#import {name, run as go} from \"lib.src\"\nn = name\nr = go\nprint missing",
            ),
            (
                "/lib.src",
                "run = function\n  return 1\nend function\nmodule.exports = {\"name\": \"lib\", \"run\": @run}",
            ),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["n", "r"]);
    let main = message_texts(&workspace, "/main.src");

    assert_eq!(types, ["string", "number"]);
    assert_eq!(main, ["Cannot find \"missing\"."]);
}

#[test]
fn should_take_the_exports_of_export_statements() {
    let workspace = load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#import {twice, count} from \"lib.src\"\nn = twice(2)\nc = count",
            ),
            (
                "/lib.src",
                "export twice = function(n)\n  return n * 2\nend function\nexport count = \"x\"",
            ),
        ],
    );

    let types = globals_in(&workspace, "/main.src", &["n", "c"]);

    assert_eq!(types, ["number", "string"]);
}
