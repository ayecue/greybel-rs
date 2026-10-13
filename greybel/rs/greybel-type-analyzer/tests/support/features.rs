//! Helpers for completion, signature help, definitions, and references.

use greybel_core::Position;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{
    Analyzer, CompletionItemKind, Database, Dialect, Location, Workspace, WorkspaceEdit,
};

use super::workspace::load;
use super::{PATH, marked, uri, with_document};

/// Returns `(label, kind, detail)` for completions at `position`.
pub fn completions(
    workspace: &Workspace<Database>,
    position: Position,
) -> Vec<(String, CompletionItemKind, String)> {
    Analyzer::new(workspace)
        .completions(&uri(PATH), position)
        .into_iter()
        .map(|item| (item.label, item.kind, item.detail))
        .collect()
}

/// Returns completion labels at `position` in the file at `path`.
pub fn completion_labels(
    workspace: &Workspace<Database>,
    path: &str,
    position: Position,
) -> Vec<String> {
    Analyzer::new(workspace)
        .completions(&uri(path), position)
        .into_iter()
        .map(|item| item.label)
        .collect()
}

/// Returns `(kind, detail)` for the completion named `name`.
pub fn find(
    items: &[(String, CompletionItemKind, String)],
    name: &str,
) -> Option<(CompletionItemKind, String)> {
    items
        .iter()
        .find(|(label, ..)| label == name)
        .map(|(_, kind, detail)| (*kind, detail.clone()))
}

/// Returns the starting `(line, column)` of each location.
pub fn starts(locations: Vec<Location>) -> Vec<(u32, u32)> {
    locations
        .into_iter()
        .map(|location| (location.range.start.line, location.range.start.character))
        .collect()
}

/// Returns `(path, line, column)` for each edit's starting position.
pub fn edited_places(edit: &WorkspaceEdit) -> Vec<(&str, u32, u32)> {
    edit.changes
        .iter()
        .flat_map(|(path, edits)| {
            edits.iter().map(move |edit| {
                (
                    path.path(),
                    edit.range.start.line,
                    edit.range.start.character,
                )
            })
        })
        .collect()
}

pub fn definition_lines(workspace: &Workspace<Database>, at: Position) -> Vec<u32> {
    Analyzer::new(workspace)
        .definition(&uri(PATH), at)
        .into_iter()
        .map(|location| location.range.start.line)
        .collect()
}

/// Variables, a parameter, and a property marked at their definitions and uses.
pub fn definitions_and_references() -> (Workspace<Database>, Vec<Position>) {
    let (source, at) = marked(
        "|count = 1
        |count = count + 1
        f = function(|value)
          return |value + |count
        end function
        Thing = {}
        Thing.|name = \"a\"
        print Thing.|name",
    );

    (with_document(Dialect::MiniScript, &source), at)
}

/// Main and library documents with markers at rename targets.
pub struct RenameProject {
    pub workspace: Workspace<Database>,
    pub main: Vec<Position>,
    pub lib: Vec<Position>,
}

pub fn rename_project() -> RenameProject {
    let (lib, lib_positions) = marked(
        "helper = function(value)\n  return value\nend function\nobj = {}\nobj.|count = 1\n",
    );
    let (main, positions) = marked("#include \"lib.src\"\nx = |helper(obj.count)\nhelper = 2\n");
    let provider = MemoryProvider::from_iter([("/lib.src", lib.as_str()), (PATH, main.as_str())]);

    RenameProject {
        workspace: load(Dialect::Greybel, &provider, &[PATH]),
        main: positions,
        lib: lib_positions,
    }
}

/// Source containing a marked native and a marked keyword.
pub fn native_and_keyword() -> (Workspace<Database>, Vec<Position>) {
    let (source, positions) = marked("x = |print\n|if true then\nend if");

    (with_document(Dialect::MiniScript, &source), positions)
}

/// Names from nested scopes and maps marked at their read sites.
pub fn scoped_definitions() -> (Workspace<Database>, Vec<Position>) {
    let (source, at) = marked(
        "tri = \"12345\"
        bar = function
          tri = 123
          foo = function
            tri = false
            foo = function
              tri = []
              print |tri
            end function
          end function
        end function
        obj = {}
        obj.test = 123
        test = function
          print obj.|test
        end function
        literal = { \"abc\": \"def\" }
        print literal.|abc
        indexed = {}
        indexed[\"abc\"] = 1
        print indexed.|abc",
    );

    (with_document(Dialect::MiniScript, &source), at)
}

/// Map members and native-result members marked after their access dots.
pub fn members_after_a_dot() -> (Workspace<Database>, Vec<Position>) {
    let (source, at) = marked(
        "Animal = {\"name\": \"cat\"}
        Animal.speak = function
          return \"meow\"
        end function
        Dog = new Animal
        Dog.|
        print get_shell.|",
    );

    (with_document(Dialect::GreyScript, &source), at)
}

/// Top-level source after a method with a local and an unknown-typed value, marked at completion positions.
pub fn in_scope_and_unknown_members() -> (Workspace<Database>, Vec<Position>) {
    let (source, at) = marked(
        "map.test = function
          foo = \"test\"
        end function
        bar = 123
        |
        unknownValue = get_custom_object.anything
        unknownValue.|",
    );

    (with_document(Dialect::GreyScript, &source), at)
}
