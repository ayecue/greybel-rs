//! Multi-entry workspace fixtures sharing libraries across folders.

use std::sync::{Arc, Mutex};

use greybel_core::{Position, Range};
use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{
    Analyzer, CompletionItemKind, Database, DiagnosticKind, Dialect, Uri, Workspace,
};
use salsa::plumbing::AsId;
use salsa::{Database as _, EventKind};

use super::{globals_in, new_workspace, uri};

pub const MAIN: &str = "/project/main.src";
pub const TOOL: &str = "/project/tool.src";
pub const UTIL: &str = "/project/lib/util.src";
pub const MATH: &str = "/project/lib/math.src";
pub const LOG: &str = "/project/shared/log.src";

pub const MAIN_SOURCE: &str = r#"#include "lib/util.src"
#include "lib/math.src"
total = add(1, 2)
label = describe(total)
note("start")
items.push(total)
names = []
append(names, "x")
if current != null then
  picked = current
end if
"#;

pub const TOOL_SOURCE: &str = r#"#include "lib/util.src"
name = describe(2)
"#;

pub const UTIL_SOURCE: &str = r#"#include "../shared/log.src"
// Formats a value.
// @param {number} value
// @returns {string}
describe = function(value)
  note("describe")
  return "" + value
end function
items = []
// @param {list<T>} out
// @param {T} item
// @mutates out
append = function(out, item)
  out.push(item)
end function
if level then current = "x"
"#;

pub const MATH_SOURCE: &str = r#"#include "util.src"
// @param {number} a
// @param {number} b
// @returns {number}
add = function(a, b)
  return a + b
end function
"#;

pub const LOG_SOURCE: &str = "level = 1
current = null
note = function(message)
  print message
end function
";

pub const INCLUDE_PREFIX: &str = "#include \"lib/";
pub const TYPE_INCLUDE_PREFIX: &str = "#include type \"lib/";
pub const IMPORT_PREFIX: &str = "#import Log from \"/project/sh";

pub fn labels(uris: &[Uri]) -> Vec<String> {
    uris.iter().map(Uri::label).collect()
}

pub fn project() -> MemoryProvider {
    MemoryProvider::from_iter([
        (MAIN, MAIN_SOURCE),
        (TOOL, TOOL_SOURCE),
        (UTIL, UTIL_SOURCE),
        (MATH, MATH_SOURCE),
        (LOG, LOG_SOURCE),
    ])
}

pub fn loaded(workspace: &mut Workspace<Database>, provider: &MemoryProvider) {
    for entry in [MAIN, TOOL] {
        workspace.load(&uri(entry), provider).expect("loads");
    }
}

pub fn workspace() -> Workspace<Database> {
    let mut workspace = new_workspace(Dialect::Greybel);

    loaded(&mut workspace, &project());

    workspace
}

pub fn load(dialect: Dialect, provider: &MemoryProvider, entries: &[&str]) -> Workspace<Database> {
    let mut workspace = new_workspace(dialect);

    for entry in entries {
        workspace.load(&uri(entry), provider).expect("loads");
    }

    workspace
}

/// Returns `(kind, message)` for every diagnostic in the file at `path`.
pub fn kinds_and_messages(
    workspace: &Workspace<Database>,
    path: &str,
) -> Vec<(DiagnosticKind, String)> {
    Analyzer::new(workspace)
        .diagnostics(&uri(path))
        .into_iter()
        .map(|diagnostic| (diagnostic.kind, diagnostic.message))
        .collect()
}

pub fn kinds(workspace: &Workspace<Database>, path: &str) -> Vec<DiagnosticKind> {
    kinds_and_messages(workspace, path)
        .into_iter()
        .map(|(kind, _)| kind)
        .collect()
}

pub fn message_texts(workspace: &Workspace<Database>, path: &str) -> Vec<String> {
    kinds_and_messages(workspace, path)
        .into_iter()
        .map(|(_, message)| message)
        .collect()
}

/// Returns the 1-based position where the zero-based `nth` occurrence of `needle` starts in `source`.
pub fn at(source: &str, needle: &str, nth: usize) -> Position {
    let offset = source
        .match_indices(needle)
        .nth(nth)
        .unwrap_or_else(|| panic!("`{needle}` occurs {} times", nth + 1))
        .0;
    let before = &source[..offset];
    let line = u32::try_from(before.matches('\n').count())
        .expect("value fits the target integer type")
        + 1;
    let column = u32::try_from(before.rsplit('\n').next().unwrap_or("").len())
        .expect("value fits the target integer type")
        + 1;

    Position::new(line, column)
}

pub fn sorted(mut files: Vec<&'static str>) -> Vec<&'static str> {
    files.sort_unstable();

    files
}

/// Returns sorted locations of `describe`'s definition and its two calls.
pub fn describe_places() -> Vec<(String, Position)> {
    let mut expected = vec![
        (MAIN.to_owned(), at(MAIN_SOURCE, "describe", 0)),
        (TOOL.to_owned(), at(TOOL_SOURCE, "describe", 0)),
        (UTIL.to_owned(), at(UTIL_SOURCE, "describe", 0)),
    ];

    expected.sort();

    expected
}

/// Returns sorted `(file, start)` pairs for references at `position`.
pub fn reference_places(
    workspace: &Workspace<Database>,
    path: &str,
    position: Position,
) -> Vec<(String, Position)> {
    let mut places: Vec<(String, Position)> = Analyzer::new(workspace)
        .references(&uri(path), position)
        .into_iter()
        .map(|location| (location.uri.label(), location.range.start))
        .collect();

    places.sort();

    places
}

/// Returns the paths of files changed by a rename at `position`.
pub fn renamed_files(
    workspace: &Workspace<Database>,
    path: &str,
    position: Position,
    name: &str,
) -> Vec<String> {
    let edit = Analyzer::new(workspace)
        .rename(&uri(path), position, name)
        .unwrap_or_else(|| panic!("renameable from {path}"));

    edit.changes
        .keys()
        .map(Uri::path)
        .map(str::to_owned)
        .collect()
}

/// Creates the project workspace with its log library removed.
pub fn without_log() -> Workspace<Database> {
    let mut workspace = workspace();

    workspace.remove_file(&uri(LOG));

    workspace
}

/// Removes the log library, queries diagnostics, then reopens it in the workspace.
pub fn log_restored() -> Workspace<Database> {
    let mut workspace = without_log();

    kinds_and_messages(&workspace, UTIL);
    kinds_and_messages(&workspace, MAIN);
    workspace.set_file(&uri(LOG), LOG_SOURCE);

    workspace
}

/// Creates a workspace where main includes missing `include` and reads `late` from it.
pub fn missing_later(include: &str) -> (Workspace<Database>, MemoryProvider) {
    let provider =
        MemoryProvider::from_iter([(MAIN, format!("#include \"{include}\"\nx = late\n"))]);

    (load(Dialect::Greybel, &provider, &[MAIN]), provider)
}

/// Queries diagnostics in [`missing_later`], then opens the missing file.
pub fn opened_later(include: &str) -> Workspace<Database> {
    let (mut workspace, _) = missing_later(include);

    kinds_and_messages(&workspace, MAIN);
    workspace.set_file(&uri("/project/later.src"), "late = 1\n");

    workspace
}

/// Queries diagnostics in [`missing_later`], then creates the missing file and reloads main.
pub fn created_later() -> Workspace<Database> {
    let (mut workspace, mut provider) = missing_later("later");

    kinds_and_messages(&workspace, MAIN);
    provider.insert("/project/later.src", "late = \"x\"\n");
    workspace.load(&uri(MAIN), &provider).expect("loads");

    workspace
}

/// Adds `extra.src` and edits main to include it and read `bonus`.
pub fn extra_included() -> Workspace<Database> {
    let mut provider = project();

    provider.insert("/project/extra.src", "bonus = \"b\"\n");

    let mut workspace = new_workspace(Dialect::Greybel);

    loaded(&mut workspace, &provider);

    let edited = format!("#include \"extra.src\"\n{MAIN_SOURCE}got = bonus\n");

    workspace.set_file(&uri(MAIN), edited);
    workspace.load(&uri(MAIN), &provider).expect("loads");

    workspace
}

/// Queries [`extra_included`], then removes the extra include from main.
pub fn extra_removed() -> Workspace<Database> {
    let mut workspace = extra_included();

    globals_in(&workspace, MAIN, &["got"]);
    kinds_and_messages(&workspace, MAIN);
    workspace.set_file(&uri(MAIN), format!("{MAIN_SOURCE}got = bonus\n"));

    workspace
}

pub fn exports_across_folders() -> Workspace<Database> {
    let provider = MemoryProvider::from_iter([
        (
            MAIN,
            "#import util from \"lib/exports.src\"\nx = util.lvl\n",
        ),
        (
            "/project/lib/exports.src",
            "#include \"../shared/log.src\"\nmodule.exports = {\"lvl\": level}\n",
        ),
        (LOG, LOG_SOURCE),
    ]);

    load(Dialect::Greybel, &provider, &[MAIN])
}

/// Records Salsa query executions by query name and file.
pub struct Executions {
    log: Arc<Mutex<Vec<salsa::DatabaseKeyIndex>>>,
}

impl Executions {
    pub fn workspace() -> (Workspace<Database>, Self) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let db = Database::with_events(move |event| {
            if let EventKind::WillExecute { database_key } = event.kind {
                sink.lock().expect("unpoisoned").push(database_key);
            }
        });

        (
            Workspace::with_database(db, Analyzer::document_options(Dialect::Greybel)),
            Self { log },
        )
    }

    /// Drains and returns query executions since the previous call as `(query name, file)` pairs.
    pub fn take(&self, workspace: &Workspace<Database>) -> Ran {
        let keys = std::mem::take(&mut *self.log.lock().expect("unpoisoned"));
        let ran = keys
            .into_iter()
            .filter_map(|key| {
                let query = workspace.db().ingredient_debug_name(key.ingredient_index());
                let path = [MAIN, TOOL, UTIL, MATH, LOG].into_iter().find(|path| {
                    workspace
                        .file(&uri(path))
                        .is_some_and(|file| file.as_id() == key.key_index())
                })?;

                Some((query.into_owned(), path))
            })
            .collect();

        Ran(ran)
    }
}

pub struct Ran(Vec<(String, &'static str)>);

impl Ran {
    /// Returns sorted unique files for which `query` executed.
    pub fn files(&self, query: &str) -> Vec<&'static str> {
        let mut files: Vec<&'static str> = self
            .0
            .iter()
            .filter(|(name, _)| {
                name.rsplit("::")
                    .next()
                    .map(|name| name.trim_end_matches('_'))
                    == Some(query)
            })
            .map(|&(_, path)| path)
            .collect();

        files.sort_unstable();
        files.dedup();

        files
    }
}

/// Requests analysis for every fixture file and returns whether all analyses exist.
pub fn analyze_all(workspace: &Workspace<Database>) -> bool {
    [MAIN, TOOL, UTIL, MATH, LOG]
        .into_iter()
        .all(|path| Analyzer::new(workspace).analysis(&uri(path)).is_some())
}

/// Returns the ordered file edits applied by [`edited`].
pub fn workspace_edits() -> [(&'static str, String); 3] {
    [
        (TOOL, format!("{TOOL_SOURCE}other = 1\n")),
        (MATH, format!("{MATH_SOURCE}two = 2\n")),
        (LOG, format!("{LOG_SOURCE}verbose = 0\n")),
    ]
}

/// Loads the project, analyzes every file once, and drains the execution log.
pub fn analyzed_once() -> (Workspace<Database>, Executions) {
    let (mut workspace, executions) = Executions::workspace();

    loaded(&mut workspace, &project());
    analyze_all(&workspace);
    executions.take(&workspace);

    (workspace, executions)
}

/// Reanalyzes [`analyzed_once`], then applies the first `count` [`workspace_edits`], analyzing and draining the
/// execution log after each edit.
pub fn edited(count: usize) -> (Workspace<Database>, Executions) {
    let (mut workspace, executions) = analyzed_once();

    analyze_all(&workspace);
    executions.take(&workspace);
    for (path, source) in workspace_edits().into_iter().take(count) {
        workspace.set_file(&uri(path), source.as_str());
        analyze_all(&workspace);
        executions.take(&workspace);
    }

    (workspace, executions)
}

/// Returns `(name, file, container)` for each workspace symbol matching `query`.
pub fn workspace_symbols(
    workspace: &Workspace<Database>,
    query: &str,
) -> Vec<(String, String, Option<String>)> {
    Analyzer::new(workspace)
        .workspace_symbols(query)
        .into_iter()
        .map(|symbol| (symbol.name, symbol.location.uri.label(), symbol.container))
        .collect()
}

/// Returns `(label, detail, edit text)` for file completions at the end of tool-file `source`.
pub fn file_completions(
    workspace: &Workspace<Database>,
    source: &str,
) -> Vec<(String, String, String)> {
    let end = Position::new(
        u32::try_from(source.lines().count()).expect("value fits the target integer type"),
        u32::try_from(source.lines().last().unwrap_or("").len())
            .expect("value fits the target integer type")
            + 1,
    );

    Analyzer::new(workspace)
        .completions(&uri(TOOL), end)
        .into_iter()
        .filter(|item| item.kind == CompletionItemKind::File)
        .map(|item| {
            let edit = item.text_edit.expect("an edit");

            (item.label, item.detail, edit.new_text)
        })
        .collect()
}

/// Sets each source as the tool file and requests completions at its end.
pub fn typed_in_tool(workspace: &mut Workspace<Database>, sources: &[&str]) {
    for source in sources {
        workspace.set_file(&uri(TOOL), *source);
        file_completions(workspace, source);
    }
}

/// Creates a GreyScript workspace whose tool file contains an `import_code` path escaping above the project root.
pub fn import_code_above_the_project() -> Workspace<Database> {
    let mut workspace = new_workspace(Dialect::GreyScript);

    loaded(&mut workspace, &project());
    workspace.set_file(&uri(TOOL), "x = 1\nimport_code(\"../project/lib/m");

    workspace
}

pub fn include_fix_provider() -> MemoryProvider {
    MemoryProvider::from_iter([
        (
            MAIN,
            "#include \"lib/math.src\"\nx = add(1, 2)\ny = describe(x)\nz = nowhere",
        ),
        (MATH, MATH_SOURCE),
        (UTIL, "level = 1\n"),
        (
            LOG,
            "describe = function(value)\n  return \"\" + value\nend function\n",
        ),
        (TOOL, "#include \"main.src\"\nextra = 1\n"),
    ])
}

/// Returns `(diagnostic kind, title, preferred, first edit text, edit start)` for the `#include` and `import_code`
/// code actions on main's `line`.
pub fn include_actions(
    dialect: Dialect,
    line: u32,
) -> Vec<(DiagnosticKind, String, bool, String, Position)> {
    let workspace = load(dialect, &include_fix_provider(), &[MAIN, LOG, TOOL]);
    let range = Range::new(Position::new(line, 1), Position::new(line, 30));

    Analyzer::new(&workspace)
        .code_actions(&uri(MAIN), range)
        .into_iter()
        .filter(|action| action.title.starts_with("Add "))
        .map(|action| {
            let edits = &action.edit.changes[&uri(MAIN)];

            (
                action.diagnostic.kind,
                action.title,
                action.is_preferred,
                edits[0].new_text.clone(),
                edits[0].range.start,
            )
        })
        .collect()
}

/// Creates a root file including all others; `b.src` uses definitions from `a.src`.
pub fn included_by_a_root() -> Workspace<Database> {
    let provider = MemoryProvider::from_iter([
        ("/app/main.src", "#include \"a.src\"\n#include \"b.src\"\n"),
        (
            "/app/a.src",
            "// Doubles.\nhelper = function(x)\n  return x * 2\nend function\n",
        ),
        (
            "/app/b.src",
            "result = helper(2)\nprint result\nprint missing\n",
        ),
    ]);

    load(
        Dialect::GreyScript,
        &provider,
        &["/app/main.src", "/app/b.src"],
    )
}

/// Creates entries where `b.src` runs after `a.src` through `main.src` and alone through `other.src`.
pub fn run_by_several_entries() -> Workspace<Database> {
    let provider = MemoryProvider::from_iter([
        ("/app/main.src", "#include \"a.src\"\n#include \"b.src\"\n"),
        ("/app/other.src", "#include \"b.src\"\n"),
        (
            "/app/a.src",
            "helper = function(x)\n  return x\nend function\n",
        ),
        (
            "/app/b.src",
            "print helper(1)\nf = function(unused)\n  return 1\nend function\n",
        ),
    ]);

    load(
        Dialect::Greybel,
        &provider,
        &["/app/main.src", "/app/other.src"],
    )
}

pub fn included_in_a_loop() -> Workspace<Database> {
    let provider = MemoryProvider::from_iter([
        (
            "/app/main.src",
            "for i in range(2)\n  #include \"step.src\"\nend for\n",
        ),
        ("/app/step.src", "if i > 0 then print last\nlast = i\n"),
        ("/app/once.src", "#include \"plain.src\"\n"),
        ("/app/plain.src", "print later\nlater = 1\n"),
    ]);

    load(
        Dialect::Greybel,
        &provider,
        &["/app/main.src", "/app/once.src"],
    )
}

/// Main includes two independent barrel files; `stray.src` is a separate entry outside those include graphs.
pub fn barrels() -> Workspace<Database> {
    let provider = MemoryProvider::from_iter([
        (
            "/app/main.src",
            "#include \"models/index.src\"\n#include \"views/index.src\"\n",
        ),
        (
            "/app/models/index.src",
            "#include \"user.src\"\n#include \"item.src\"\n",
        ),
        ("/app/views/index.src", "#include \"page.src\"\n"),
        (
            "/app/models/user.src",
            "User = {}\nUser.name = \"\"\nUser.greet = function\n  return \"hi \" + self.name\nend function\n",
        ),
        ("/app/models/item.src", "Item = {}\nItem.price = 1\n"),
        (
            "/app/views/page.src",
            "u = new User\ntext = u.greet\ntotal = Item.price * 2\nprint unknownThing\n",
        ),
        ("/app/stray.src", "print User\n"),
    ]);

    load(
        Dialect::Greybel,
        &provider,
        &["/app/main.src", "/app/stray.src"],
    )
}
