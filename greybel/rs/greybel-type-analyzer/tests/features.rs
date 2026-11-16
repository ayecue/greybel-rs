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
fn should_rename_a_parameter_in_param_tags() {
    let (source, at) = marked(
        "// Adds.\n// @param {number} [c|ount] How many.\n// @param {string} label\nadd = function(count, label)\n  return |count + label\nend function\n// @param {number} count\nother = function(count)\n  return count\nend function\nadd(1, \"a\")\nother(2)",
    );
    let workspace = with_document(Dialect::Greybel, &source);
    let analyzer = Analyzer::new(&workspace);

    let from_usage = analyzer
        .rename(&uri(PATH), at[1], "total")
        .expect("a parameter");
    let from_tag = analyzer
        .rename(&uri(PATH), at[0], "total")
        .expect("a tag name");
    let prepared = analyzer
        .prepare_rename(&uri(PATH), at[0])
        .expect("a tag name");

    let expected = [(PATH, 2, 21), (PATH, 4, 16), (PATH, 5, 10)];

    assert_eq!(edited_places(&from_usage), expected);
    assert_eq!(edited_places(&from_tag), expected);
    assert_eq!(
        (prepared.start, prepared.end),
        (Position::new(2, 21), Position::new(2, 26))
    );
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
            (
                "Foo".to_owned(),
                SymbolKind::Variable,
                vec!["New".to_owned()]
            ),
            ("total".to_owned(), SymbolKind::Variable, vec![]),
            ("list".to_owned(), SymbolKind::Variable, vec![]),
            ("list[0]".to_owned(), SymbolKind::Property, vec![]),
        ]
    );

    let new = &symbols[0].children[0];
    let inner: Vec<&str> = new
        .children
        .iter()
        .map(|child| child.name.as_str())
        .collect();

    assert_eq!(inner, ["inner", "deep"]);
    assert_eq!(
        (symbols[0].range.start.line, symbols[0].range.end.line),
        (1, 7)
    );
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

#[test]
fn should_hover_the_keys_of_the_custom_object_with_its_signature() {
    let source = "get_custom_object.count = 1\nset = function\n  get_custom_object.name = \"n\"\nend function\nx = get_custom_object";
    let workspace = with_document(Dialect::GreyScript, source);

    let hover = Analyzer::new(&workspace)
        .hover(&uri(PATH), Position::new(5, 6))
        .map(|hover| hover.text);

    assert_eq!(
        hover.as_deref(),
        Some(
            "get_custom_object: function(): map {\n  \"count\": number,\n  \"name\": string,\n  [any]: any\n}"
        )
    );
}

#[test]
fn should_hover_the_keys_of_a_returned_map_but_not_of_a_custom_type() {
    let source = "makeUser = function(n)\n  return {\"name\": n}\nend function\n// @type Point\n// @property {number} x\nPoint = {\"x\": 1}\n// @return {Point}\nmakePoint = function\n  return new Point\nend function";
    let workspace = with_document(Dialect::GreyScript, source);
    let hover = |line| {
        Analyzer::new(&workspace)
            .hover(&uri(PATH), Position::new(line, 2))
            .map(|hover| hover.text)
    };

    let user = hover(1);
    let point = hover(8);

    assert_eq!(
        user.as_deref(),
        Some("makeUser: function(n: any): map {\n  \"name\": any\n}")
    );
    assert_eq!(point.as_deref(), Some("makePoint: function(): Point"));
}

#[test]
fn should_find_every_key_assignment_as_the_definition_of_the_custom_object() {
    let source = "get_custom_object.count = 1\nset = function\n  get_custom_object[\"name\"] = \"n\"\nend function\nget_custom_object.count = 2\nx = get_custom_object";
    let workspace = with_document(Dialect::GreyScript, source);

    let lines = definition_lines(&workspace, Position::new(6, 6));

    assert_eq!(lines, [1, 3, 5]);
}

#[test]
fn should_mark_deprecated_functions_in_hovers_and_completions() {
    let source = "// Adds two numbers.\n// @deprecated Use add instead.\nsum = function(a, b)\n  return a + b\nend function\nx = sum(1, 2)\nsu";
    let workspace = with_document(Dialect::Greybel, source);
    let analyzer = Analyzer::new(&workspace);

    let hover = analyzer
        .hover(&uri(PATH), Position::new(6, 6))
        .expect("a hover");
    let item = analyzer
        .completions(&uri(PATH), Position::new(7, 3))
        .into_iter()
        .find(|item| item.label == "sum")
        .expect("a completion for sum");

    assert_eq!(
        hover
            .documentation
            .and_then(|documentation| documentation.description),
        Some("Deprecated: Use add instead.\n\nAdds two numbers.".to_owned())
    );
    assert!(item.deprecated);
}

#[test]
fn should_complete_the_string_keys_of_a_map_inside_brackets() {
    let (source, at) =
        marked("m = {\"name\": \"x\", \"port\": 22}\nprint m[\"p|\"]\nprint m[\"name\"] + |");
    let workspace = with_document(Dialect::Greybel, &source);
    let analyzer = Analyzer::new(&workspace);

    let keys = analyzer.completions(&uri(PATH), at[0]);
    let after = completion_labels(&workspace, PATH, at[1]);
    let edits: Vec<_> = keys
        .iter()
        .map(|item| {
            (
                item.label.as_str(),
                item.text_edit.as_ref().map(|edit| edit.range.start),
            )
        })
        .collect();

    assert_eq!(
        edits,
        [
            ("name", Some(Position::new(2, 10))),
            ("port", Some(Position::new(2, 10)))
        ]
    );
    assert!(after.contains(&"print".to_owned()), "{after:?}");
}

#[test]
fn should_list_declared_types_as_symbols() {
    let source = "x = 1\n// @type Point\n// @property {number} n\n// @function length\n// @return {number}\ny = 2";
    let workspace = with_document(Dialect::Greybel, source);
    let analyzer = Analyzer::new(&workspace);

    let symbols = analyzer.document_symbols(&uri(PATH));
    let outline: Vec<_> = symbols
        .iter()
        .map(|symbol| {
            let children: Vec<_> = symbol
                .children
                .iter()
                .map(|child| (child.name.as_str(), child.kind, child.selection_range.start))
                .collect();

            (
                symbol.name.as_str(),
                symbol.kind,
                symbol.selection_range.start,
                children,
            )
        })
        .collect();
    let found = analyzer.workspace_symbols("Poin");

    assert_eq!(
        outline,
        [
            ("x", SymbolKind::Variable, Position::new(1, 1), vec![]),
            (
                "Point",
                SymbolKind::Type,
                Position::new(2, 10),
                vec![
                    ("n", SymbolKind::Property, Position::new(3, 23)),
                    ("length", SymbolKind::Function, Position::new(4, 14)),
                ]
            ),
            ("y", SymbolKind::Variable, Position::new(6, 1), vec![]),
        ]
    );
    assert_eq!(found[0].name, "Point");
}

#[test]
fn should_nest_member_assignments_under_the_symbol_of_their_owner() {
    let source = "App.utils = {}\nApp.utils.format = function(x)\nend function\nApp.utils.trim = 1\nOther.x = 2\nm[\"k\"].y = 3";
    let workspace = with_document(Dialect::GreyScript, source);

    let symbols = Analyzer::new(&workspace).document_symbols(&uri(PATH));
    let outline: Vec<(&str, Vec<&str>)> = symbols
        .iter()
        .map(|symbol| {
            let children = symbol
                .children
                .iter()
                .map(|child| child.name.as_str())
                .collect();

            (symbol.name.as_str(), children)
        })
        .collect();

    assert_eq!(
        outline,
        [
            ("App.utils", vec!["format", "trim"]),
            ("Other.x", vec![]),
            ("m[\"k\"].y", vec![]),
        ]
    );
}

#[test]
fn should_outline_classes_with_their_members() {
    let source = "class Animal\n  tags = []\n  constructor(name)\n    self.name = name\n  end function\nend class\nclass Dog extends Animal\n  sound = \"woof\"\n  bark()\n    return self.sound\n  end function\nend class";
    let workspace = with_document(Dialect::Greybel, source);

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
            (
                "Animal".to_owned(),
                SymbolKind::Class,
                vec!["tags".to_owned(), "constructor".to_owned()]
            ),
            (
                "Dog".to_owned(),
                SymbolKind::Class,
                vec!["sound".to_owned(), "bark".to_owned()]
            ),
        ]
    );
}

#[test]
fn should_rename_a_class_at_its_name_and_uses_only() {
    let source = "class Dog\n  sound = \"woof\"\n  bark()\n    return 1\n  end function\nend class\nd = Dog.create\nx = Dog.sound";
    let workspace = with_document(Dialect::Greybel, source);

    let edits = Analyzer::new(&workspace)
        .rename(&uri(PATH), Position::new(1, 8), "Hound")
        .expect("renameable");

    assert_eq!(
        edited_places(&edits),
        [
            ("/main.src", 1, 7),
            ("/main.src", 7, 5),
            ("/main.src", 8, 5)
        ]
    );
}

#[test]
fn should_keep_the_key_of_a_shorthand_when_renaming() {
    let (source, at) =
        marked("name = \"a\"\nm = {name}\n{name} = {\"name\": 1}\nprint |name\nprint m.na|me");
    let workspace = with_document(Dialect::Greybel, &source);
    let analyzer = Analyzer::new(&workspace);
    let texts = |position, name: &str| {
        let edits = analyzer
            .rename(&uri(PATH), position, name)
            .expect("renameable");
        let mut texts: Vec<String> = edits
            .changes
            .values()
            .flatten()
            .map(|edit| edit.new_text.clone())
            .collect();

        texts.sort();

        texts
    };

    let variable = texts(at[0], "label");
    let key = texts(at[1], "title");

    assert_eq!(
        variable,
        ["\"name\": label", "label", "label", "name as label"]
    );
    assert_eq!(key, ["\"title\": name", "title"]);
}

#[test]
fn should_go_to_the_class_name_from_generated_members() {
    let (source, at) = marked(
        "class |Dog\n  tags = []\n  bark()\n    return 1\n  end function\nend class\nd = Dog.cr|eate\nc = @Dog.con|structor",
    );
    let workspace = with_document(Dialect::Greybel, &source);
    let analyzer = Analyzer::new(&workspace);

    let create = starts(analyzer.definition(&uri(PATH), at[1]));
    let constructor = starts(analyzer.definition(&uri(PATH), at[2]));

    assert_eq!(create, [(at[0].line, at[0].character)]);
    assert_eq!(constructor, [(at[0].line, at[0].character)]);
}

#[test]
fn should_hover_class_fields_in_the_class_body() {
    let (source, at) = marked(
        "class Dog\n  t|ags = []\n  static so|und = \"woof\"\n  private a|ge = 1\n  constructor(name)\n    self.name = name\n  end function\nend class",
    );
    let workspace = with_document(Dialect::Greybel, &source);
    let analyzer = Analyzer::new(&workspace);
    let text = |position| analyzer.hover(&uri(PATH), position).map(|hover| hover.text);

    assert_eq!(text(at[0]).as_deref(), Some("tags: list<unknown>"));
    assert_eq!(text(at[1]).as_deref(), Some("sound: string"));
    assert_eq!(text(at[2]).as_deref(), Some("age: number"));
}

#[test]
fn should_hover_type_names_in_code_and_comments() {
    let (source, at) = marked(
        "// @type Po|int\n// @property {num|ber} x\n\n// @typedef {list<number>} Id\n\ntype Re|po extends map\n  name: string\nend type\nf = function(r: Rep|o, i: I|d): Point\nend function\n// @param {Re|po} r\ng = function(r)\nend function",
    );
    let workspace = with_document(Dialect::Greybel, &source);
    let analyzer = Analyzer::new(&workspace);
    let text = |position| analyzer.hover(&uri(PATH), position).map(|hover| hover.text);
    let repo = "type Repo {\n  \"name\": string\n}";

    assert_eq!(
        text(at[0]).as_deref(),
        Some("type Point {\n  \"x\": number\n}")
    );
    assert_eq!(text(at[1]).as_deref(), Some("type number"));
    assert_eq!(text(at[2]).as_deref(), Some(repo));
    assert_eq!(text(at[3]).as_deref(), Some(repo));
    assert_eq!(text(at[4]).as_deref(), Some("type Id = list<number>"));
    assert_eq!(text(at[5]).as_deref(), Some(repo));
}
