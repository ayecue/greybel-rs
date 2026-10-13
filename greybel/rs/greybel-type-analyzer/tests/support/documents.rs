//! Fixtures for documents importing files with `#include`, `#import`, and `import_code`.

use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{Database, Dialect, Workspace};

use super::{globals_in, uri};

/// Creates a workspace with `/main.src` loaded from `files`.
pub fn load(dialect: Dialect, files: &[(&str, &str)]) -> Workspace<Database> {
    let provider: MemoryProvider = files.iter().copied().collect();

    super::workspace::load(dialect, &provider, &["/main.src"])
}

pub fn shared_include() -> Workspace<Database> {
    load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#include \"lib.src\"\nout = helper\ncount = shared",
            ),
            (
                "/lib.src",
                "helper = function\n  return \"x\"\nend function\nshared = 1",
            ),
        ],
    )
}

pub fn module_import() -> Workspace<Database> {
    load(
        Dialect::Greybel,
        &[
            (
                "/main.src",
                "#import lib from \"lib.src\"\nname = lib.name\nresult = lib.run\nprint secret",
            ),
            (
                "/lib.src",
                "secret = 1\nrun = function\n  return secret\nend function\nmodule.exports = {\"name\": \"lib\", \"run\": @run}",
            ),
        ],
    )
}

pub fn dependency() -> Workspace<Database> {
    load(
        Dialect::Greybel,
        &[
            ("/main.src", "#include \"lib.src\"\nout = value"),
            ("/lib.src", "value = 1"),
        ],
    )
}

/// Queries [`dependency`], then edits the included file.
pub fn dependency_edited() -> Workspace<Database> {
    let mut workspace = dependency();

    globals_in(&workspace, "/main.src", &["out"]);
    workspace.set_file(&uri("/lib.src"), "value = \"changed\"");

    workspace
}

/// Queries [`dependency_edited`], then edits the including file.
pub fn dependent_edited() -> Workspace<Database> {
    let mut workspace = dependency_edited();

    globals_in(&workspace, "/main.src", &["out"]);
    workspace.set_file(&uri("/main.src"), "#include \"lib.src\"\nout = [value]");

    workspace
}

pub fn missing_imports() -> Workspace<Database> {
    load(
        Dialect::Greybel,
        &[(
            "/main.src",
            "#import HelloWord from \"library/hello-world.src\";\n#import HelloName from \"library/hello-name.src\";\nHelloWord\nHelloName(\"Joe\")",
        )],
    )
}
