//! Completion, signature help, definition and references, with 1-based positions.

use greybel_core::Position;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{Analyzer, CompletionItemKind, Dialect, SymbolKind};

use crate::support::features::{
    completion_labels, completions, definition_lines, definitions_and_references, edited_places,
    find, in_scope_and_unknown_members, members_after_a_dot, native_and_keyword, rename_project,
    scoped_definitions, starts,
};
use crate::support::workspace::load;
use crate::support::{PATH, marked, uri, with_document};

#[test]
fn should_complete_members_after_a_dot() {
    let (workspace, at) = members_after_a_dot();

    let members = completions(&workspace, at[0]);

    assert_eq!(
        members[..2],
        [
            (
                "name".to_owned(),
                CompletionItemKind::Property,
                "string".to_owned()
            ),
            (
                "speak".to_owned(),
                CompletionItemKind::Method,
                "function(): string".to_owned()
            ),
        ]
    );
    assert!(members.iter().any(|(label, ..)| label == "hasIndex"));
}

#[test]
fn should_complete_members_of_a_native_result_after_a_dot() {
    let (workspace, at) = members_after_a_dot();

    let shell = completion_labels(&workspace, PATH, at[1]);

    assert!(shell.contains(&"host_computer".to_owned()), "{shell:?}");
    assert!(shell.contains(&"hasIndex".to_owned()), "{shell:?}");
}

#[test]
fn should_complete_names_in_scope_nearest_first() {
    let (source, at) = marked(
        "count = 1
        f = function(count, extra)
          local = \"x\"
          |
        end function
        |",
    );
    let workspace = with_document(Dialect::MiniScript, &source);

    let inside = completions(&workspace, at[0]);
    let outside = completions(&workspace, at[1]);

    assert_eq!(
        find(&inside, "count"),
        Some((CompletionItemKind::Variable, "any".to_owned()))
    );
    assert_eq!(
        find(&inside, "local"),
        Some((CompletionItemKind::Variable, "string".to_owned()))
    );
    assert_eq!(
        find(&inside, "print").map(|(kind, _)| kind),
        Some(CompletionItemKind::Function)
    );
    assert_eq!(
        find(&outside, "count"),
        Some((CompletionItemKind::Variable, "number".to_owned()))
    );
    assert_eq!(find(&outside, "local"), None);
}

#[test]
fn should_follow_the_arguments_in_signature_help() {
    let (source, at) = marked(
        "// Adds.
        // @param {number} a - The first.
        add = function(a, b = 1)
          return a + b
        end function
        x = add(|1, |
        y = get_shell(\"root\", |)
        Thing = {}
        Thing.run = function(self, times)
        end function
        Thing.run(|",
    );
    let workspace = with_document(Dialect::GreyScript, &source);
    let help = |position| {
        Analyzer::new(&workspace)
            .signature_help(&uri(PATH), position)
            .expect("in a call")
    };

    let first = help(at[0]);
    let second = help(at[1]);
    let native = help(at[2]);
    let method = help(at[3]);

    assert_eq!(first.label, "add(a: number, b: number = 1): number");
    assert_eq!(first.active_parameter, 0);
    assert_eq!(
        first.parameters[0].documentation.as_deref(),
        Some("The first.")
    );
    assert_eq!(
        first
            .documentation
            .and_then(|doc| doc.description)
            .as_deref(),
        Some("Adds.")
    );
    assert_eq!(second.active_parameter, 1);
    assert_eq!(
        native.label,
        "get_shell(user: string = \"\", password: string = \"\"): shell or null or string"
    );
    assert_eq!(native.active_parameter, 1);
    assert_eq!(method.label, "run(times: any): null");
}

#[test]
fn should_find_definitions_of_variables_parameters_and_properties() {
    let (workspace, at) = definitions_and_references();

    let count = starts(Analyzer::new(&workspace).definition(&uri(PATH), at[4]));
    let value = starts(Analyzer::new(&workspace).definition(&uri(PATH), at[3]));
    let name = starts(Analyzer::new(&workspace).definition(&uri(PATH), at[6]));

    assert_eq!(
        count,
        [(at[0].line, at[0].character), (at[1].line, at[1].character)]
    );
    assert_eq!(value, [(at[2].line, at[2].character)]);
    assert_eq!(name, [(at[5].line, at[5].character)]);
}

#[test]
fn should_find_references_of_variables_and_properties() {
    let (workspace, at) = definitions_and_references();

    let count = Analyzer::new(&workspace).references(&uri(PATH), at[0]);
    let name = Analyzer::new(&workspace).references(&uri(PATH), at[5]);

    assert_eq!(count.len(), 4);
    assert_eq!(name.len(), 2);
}

#[test]
fn should_find_definitions_in_other_files() {
    let provider: MemoryProvider = [
        ("/main.src", "#include \"lib.src\"\nprint helper"),
        ("/lib.src", "x = 1\nhelper = function\nend function"),
    ]
    .into_iter()
    .collect();
    let workspace = load(Dialect::Greybel, &provider, &["/main.src"]);

    let locations = Analyzer::new(&workspace).definition(&uri("/main.src"), Position::new(2, 8));

    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri, uri("/lib.src"));
    assert_eq!(locations[0].range.start, Position::new(2, 1));
}

#[test]
fn should_rename_every_reference_across_files() {
    let project = rename_project();

    let edits = Analyzer::new(&project.workspace)
        .rename(&uri(PATH), project.main[0], "assist")
        .expect("renameable");

    let places = edited_places(&edits);

    assert_eq!(
        places,
        [("/lib.src", 1, 1), ("/main.src", 2, 5), ("/main.src", 3, 1)]
    );
    assert!(
        edits
            .changes
            .values()
            .flatten()
            .all(|edit| edit.new_text == "assist")
    );
}

#[test]
fn should_prepare_a_rename_of_the_name_under_the_cursor() {
    let project = rename_project();

    let range = Analyzer::new(&project.workspace)
        .prepare_rename(&uri(PATH), project.main[0])
        .expect("a name");

    assert_eq!((range.start.character, range.end.character), (5, 11));
}

#[test]
fn should_rename_a_property_across_files() {
    let project = rename_project();

    let edits = Analyzer::new(&project.workspace)
        .rename(&uri("/lib.src"), project.lib[0], "total")
        .expect("a member");

    let places = edited_places(&edits);

    assert_eq!(places, [("/lib.src", 5, 5), ("/main.src", 2, 16)]);
}

#[test]
fn should_refuse_to_rename_to_a_keyword_or_no_name() {
    let project = rename_project();
    let at = project.main[0];

    let keyword = Analyzer::new(&project.workspace).rename(&uri(PATH), at, "end");
    let two_words = Analyzer::new(&project.workspace).rename(&uri(PATH), at, "two words");
    let empty = Analyzer::new(&project.workspace).rename(&uri(PATH), at, "");

    assert!(keyword.is_none());
    assert!(two_words.is_none());
    assert!(empty.is_none());
}

#[test]
fn should_not_prepare_renaming_a_native() {
    let (workspace, positions) = native_and_keyword();

    let range = Analyzer::new(&workspace).prepare_rename(&uri(PATH), positions[0]);

    assert!(range.is_none());
}

#[test]
fn should_not_rename_a_keyword() {
    let (workspace, positions) = native_and_keyword();

    let edits = Analyzer::new(&workspace).rename(&uri(PATH), positions[1], "y");

    assert!(edits.is_none());
}

#[test]
fn should_outline_assignments_as_document_symbols() {
    let source = "Foo = {}\nFoo.New = function(a)\n  inner = a\n  if a then\n    deep = 1\n  end if\nend function\nfor i in range(3)\n  total = i\nend for\nlist = [1]\nlist[0] = 2";
    let workspace = with_document(Dialect::MiniScript, source);

    let symbols = Analyzer::new(&workspace).document_symbols(&uri(PATH));

    let outline: Vec<(String, SymbolKind, Vec<String>)> = symbols
        .iter()
        .map(|symbol| {
            let children = symbol
                .children
                .iter()
                .map(|child| child.name.clone())
                .collect();

            (symbol.name.clone(), symbol.kind, children)
        })
        .collect();

    assert_eq!(
        outline,
        [
            ("Foo".to_owned(), SymbolKind::Variable, vec![]),
            (
                "Foo.New".to_owned(),
                SymbolKind::Function,
                vec!["inner".to_owned(), "deep".to_owned()]
            ),
            ("total".to_owned(), SymbolKind::Variable, vec![]),
            ("list".to_owned(), SymbolKind::Variable, vec![]),
            ("list[0]".to_owned(), SymbolKind::Property, vec![]),
        ]
    );

    let new = &symbols[1];

    assert_eq!((new.range.start.line, new.range.end.line), (2, 7));
    assert_eq!(
        (
            new.selection_range.start.character,
            new.selection_range.end.character
        ),
        (1, 8)
    );
}

#[test]
fn should_find_definitions_by_scope_and_entry() {
    let (workspace, at) = scoped_definitions();

    let lines: Vec<Vec<u32>> = at
        .iter()
        .map(|&position| definition_lines(&workspace, position))
        .collect();

    assert_eq!(lines[0], [7]);
    assert_eq!(lines[1], [13]);
    assert_eq!(lines[2], [17]);
    assert_eq!(lines[3], [20]);
}

#[test]
fn should_find_references_of_a_map_literal_entry() {
    let (workspace, at) = scoped_definitions();

    let references = Analyzer::new(&workspace).references(&uri(PATH), at[2]);

    assert_eq!(references.len(), 2);
}

#[test]
fn should_find_definitions_of_methods_in_their_classes() {
    let source = "// @type Bar
        Bar = {}
        Bar.test = function(functionName)
          return self
        end function
        // @type Moo
        Moo = new Bar
        Moo.test = function
        end function
        // @type Foo
        Foo = new Bar
        // @return {Foo}
        Foo.New = function(message)
          return new Foo
        end function
        // @return {Moo|Foo}
        pick = function
        end function
        test = Foo.New
        test.test
        either = pick
        either.test
        Foo = \"was\"
        print Foo";
    let workspace = with_document(Dialect::Greybel, source);

    let instance = definition_lines(&workspace, Position::new(20, 14));
    let either = definition_lines(&workspace, Position::new(22, 16));
    let reassigned = definition_lines(&workspace, Position::new(24, 15));

    assert_eq!(instance, [3]);
    assert_eq!(either, [3, 8]);
    assert_eq!(reassigned, [11, 23]);
}

#[test]
fn should_complete_what_is_in_scope() {
    let (workspace, at) = in_scope_and_unknown_members();

    let top = completion_labels(&workspace, PATH, at[0]);

    assert!(top.iter().any(|name| name == "bar"));
    assert!(!top.iter().any(|name| name == "foo" || name == "test"));
}

#[test]
fn should_complete_map_functions_not_custom_methods_on_an_unknown_value() {
    let (workspace, at) = in_scope_and_unknown_members();

    let members = completion_labels(&workspace, PATH, at[1]);

    assert!(members.iter().any(|name| name == "hasIndex"));
    assert!(!members.iter().any(|name| name == "test"));
}
