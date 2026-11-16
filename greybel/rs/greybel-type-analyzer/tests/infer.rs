//! Single-document inference across types, members and calls.

use std::fmt::Write as _;

use greybel_core::Position;
use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect};

use crate::support::diagnostics::{messages_of_kind, owned};
use crate::support::infer::FUNCTION_LOCALS;
use crate::support::{
    PATH, global, globals, greyscript, locals, main_global, marked, miniscript, uri, with_document,
};

#[test]
fn should_type_a_variable_as_everything_assigned_to_it() {
    let once = "test = \"123\"";
    let twice = "test = \"123\"\ntest = 123";

    let once = miniscript(once, &["test"]);
    let twice = miniscript(twice, &["test"]);

    assert_eq!(once, ["string"]);
    assert_eq!(twice, ["number or string"]);
}

#[test]
fn should_widen_properties_like_variables() {
    let once = "test = {}\ntest.foo = 123\nbar = test.foo";
    let twice = "test = {}\ntest.foo = 123\ntest.foo = []\nbar = test.foo";
    let branch = "test = {}\ntest.foo = 123\nif rnd then test.foo = []\nbar = test.foo";
    let member = "test = {}\ntest.foo = []\nbar = test.foo.hasIndex";

    let once = miniscript(once, &["bar"]);
    let twice = miniscript(twice, &["bar"]);
    let branch = miniscript(branch, &["bar"]);
    let member = miniscript(member, &["bar"]);

    assert_eq!(once, ["number"]);
    assert_eq!(twice, ["list<unknown>"]);
    assert_eq!(branch, ["list<unknown> or number"]);
    assert_eq!(member, ["number"]);
}

#[test]
fn should_read_literal_keys_as_properties() {
    let strings = "test = {}\ntest[\"foo\"] = \"hello\"\ntest[\"foo bar\"] = 1\na = test.foo\nb = test[\"foo bar\"]";
    let numbers = "test = {}\ntest[222] = \"hello\"\ntest[454] = {}\na = test[222]\nb = test[454]";

    let strings = miniscript(strings, &["a", "b"]);
    let numbers = miniscript(numbers, &["a", "b"]);

    assert_eq!(strings, ["string", "number"]);
    assert_eq!(numbers, ["string", "map"]);
}

#[test]
fn should_read_values_under_keys_of_a_computed_keys_type() {
    let source = "test = {\"name\": \"test\", 123: \"bar\", 234: [1]}\nkey = 1\nfoo = test[key]\nkey2 = \"a\"\nbar = test[key2]";

    let types = miniscript(source, &["foo", "bar"]);

    assert_eq!(types, ["list<number> or string", "string"]);
}

#[test]
fn should_call_named_functions_unless_referenced() {
    let source = "test = {}
        test.testFn = function(a = 2, b = \"test\")
            return \"x\"
        end function
        b = test[\"testFn\"]
        d = @test[\"testFn\"]
        e = @test.testFn
        f = @test.testFn()
        g = test.testFn
        h = test.testFn()";

    let types = miniscript(source, &["b", "d", "e", "f", "g", "h"]);

    let function = "function(a: number = 2, b: string = \"test\"): string";

    assert_eq!(
        types,
        [function, function, function, "string", "string", "string"]
    );
}

#[test]
fn should_extend_values_with_their_prototype_maps() {
    let source = "string.test = function
            return 1
        end function
        number.test = function
            return \"x\"
        end function
        map.test = function(foo = 123)
        end function
        a = \"test\".test
        b = (123).test
        c = (true).test
        d = (1 + 2).test
        e = (1 + \"test\").test
        f = @map.test
        g = map.test
        h = @{}.test
        i = {}.test";

    let types = miniscript(source, &["a", "b", "c", "d", "e", "f", "g", "h", "i"]);

    let function = "function(foo: number = 123): null";

    assert_eq!(
        types,
        [
            "number", "string", "string", "string", "number", function, "null", function, "null"
        ]
    );
}

#[test]
fn should_take_natives_from_the_api() {
    let source = "shell = get_shell
        computer = shell.host_computer
        files = computer.File(\"/\").get_files
        name = files[0].name
        size = \"abc\".len
        parts = \"a b\".split(\" \")
        method = @shell.host_computer
        user = params";

    let types = greyscript(
        source,
        &[
            "shell", "computer", "files", "name", "size", "parts", "method", "user",
        ],
    );

    assert_eq!(
        types,
        [
            "null or shell or string",
            "computer",
            "list<file> or null or string",
            "null or string",
            "number",
            "list<string> or null or string",
            "function(): computer",
            "list<string>"
        ]
    );
}

#[test]
fn should_type_operators() {
    let source = "a = \"123\" isa string
        b = 1 + 2
        c = \"a\" + 1
        d = [1] + [\"x\"]
        e = \"ab\" * 2
        f = not 1
        g = 1 < 2 < 3
        h = [1][1:]";

    let types = miniscript(source, &["a", "b", "c", "d", "e", "f", "g", "h"]);

    assert_eq!(
        types,
        [
            "number",
            "number",
            "string",
            "list<number or string>",
            "string",
            "number",
            "number",
            "list<number>"
        ]
    );
}

#[test]
fn should_scope_variables_to_their_function() {
    let workspace = with_document(Dialect::MiniScript, FUNCTION_LOCALS);
    let analysis = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("added");

    let locals = locals(analysis, 0, &["foo", "bar"]);

    assert_eq!(locals, ["number or string", "string"]);
}

#[test]
fn should_type_a_global_by_what_a_function_returns() {
    let source = FUNCTION_LOCALS;

    let result = miniscript(source, &["result"]);

    assert_eq!(result, ["string"]);
}

#[test]
fn should_not_define_the_locals_of_a_function_globally() {
    let workspace = with_document(Dialect::MiniScript, FUNCTION_LOCALS);
    let analysis = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("added");

    let foo = analysis.global_type("foo");

    assert!(foo.is_none());
}

#[test]
fn should_read_the_outer_scope_and_the_globals() {
    let source = "x = 1
        outerFn = function
            y = \"a\"
            innerFn = function
                return [x, y]
            end function
            return innerFn
        end function
        result = outerFn
        explicit = globals.x
        globals.created = {}";

    let types = miniscript(source, &["result", "explicit", "created"]);

    assert_eq!(types, ["list<number or string>", "number", "map"]);
}

#[test]
fn should_make_self_the_map_a_method_is_stored_on() {
    let source = "Animal = {}
        Animal.name = \"animal\"
        Animal.describe = function
            self.legs = 4
            return self.name
        end function
        Dog = new Animal
        Dog.bark = function
            return super.describe
        end function
        name = Dog.describe
        legs = Dog.legs
        parent = Dog.__isa
        bark = Dog.bark";

    let types = miniscript(source, &["name", "legs", "parent", "bark"]);

    assert_eq!(types, ["string", "number", "map", "string"]);
}

#[test]
fn should_bind_the_items_of_loops() {
    let source = "for item in [1, 2]
            a = item
        end for
        for character in \"abc\"
            b = character
        end for
        for pair in {\"x\": 1}
            c = pair.key
            d = pair.value
        end for
        e = __item_idx";

    let types = miniscript(source, &["a", "b", "c", "d", "e"]);

    assert_eq!(types, ["number", "string", "string", "number", "number"]);
}

#[test]
fn should_converge_on_recursion() {
    let source = "count = function(n)
            if n <= 0 then return 0
            return count(n - 1) + 1
        end function
        result = count(3)
        nested = []
        nested = [nested]
        cycle = cycle";

    let types = miniscript(source, &["result", "nested", "cycle"]);

    assert_eq!(types, ["number", "list<list<unknown>>", "unknown"]);
}

#[test]
fn should_analyze_past_syntax_errors() {
    let workspace = with_document(
        Dialect::Greybel,
        "test = function(abc =)\n\nend function\n\ntest\nfoo = 123",
    );

    let foo = main_global(&workspace, "foo");

    assert_eq!(foo, "number");
}

#[test]
fn should_report_syntax_errors() {
    let workspace = with_document(
        Dialect::Greybel,
        "test = function(abc =)\n\nend function\n\ntest\nfoo = 123",
    );

    let diagnostics = Analyzer::new(&workspace).diagnostics(&uri(PATH));

    assert!(!diagnostics.is_empty());
}

#[test]
fn should_report_unresolved_names() {
    let workspace = with_document(Dialect::MiniScript, "a = 1\nprint b + a\nc = str(a)");

    let diagnostics: Vec<(String, u32)> = Analyzer::new(&workspace)
        .diagnostics(&uri(PATH))
        .into_iter()
        .map(|diagnostic| (diagnostic.message, diagnostic.range.start.line))
        .collect();

    assert_eq!(diagnostics, [("Cannot find \"b\".".to_owned(), 2)]);
}

#[test]
fn should_type_greybel_directives() {
    let source = "#import lib from \"lib.src\"\nline = #line\nfile = #filename\nbits = 1 << 2";

    let types = globals(Dialect::Greybel, source, &["lib", "line", "file", "bits"]);

    assert_eq!(types, ["any", "number", "string", "number"]);
}

#[test]
fn should_type_string_templates_and_resolve_their_expressions() {
    let source = "n = 1\nmsg = #\"a{n}b{#\"{m}\"}\"";
    let workspace = with_document(Dialect::Greybel, source);

    let types = globals(Dialect::Greybel, source, &["msg"]);
    let diagnostics: Vec<(String, u32)> = Analyzer::new(&workspace)
        .diagnostics(&uri(PATH))
        .into_iter()
        .map(|diagnostic| (diagnostic.message, diagnostic.range.start.line))
        .collect();

    assert_eq!(types, ["string"]);
    assert_eq!(diagnostics, [("Cannot find \"m\".".to_owned(), 2)]);
}

#[test]
fn should_type_enums_as_maps_of_their_values() {
    let source = "enum Color\n  Red\n  Name = \"n\"\nend enum\nred = Color.Red\nname = Color.Name\nx = Color.Redd";
    let workspace = with_document(Dialect::Greybel, source);

    let types = globals(Dialect::Greybel, source, &["red", "name"]);
    let unknown: Vec<(String, u32)> = Analyzer::new(&workspace)
        .diagnostics(&uri(PATH))
        .into_iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::UnknownMember)
        .map(|diagnostic| (diagnostic.message, diagnostic.range.start.line))
        .collect();

    assert_eq!(types, ["number", "string"]);
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].1, 7);
}

#[test]
fn should_type_spreads_and_map_shorthands() {
    let source = "a = [1]\nd = {\"x\": \"s\"}\nname = 2\nl = [...a, \"t\"]\nm = {...d, name}\nx = m.x\nn = m.name\nu = m.zz\no = {...rnd, \"k\": 1}\nv = o.zz";
    let workspace = with_document(Dialect::Greybel, source);

    let types = globals(Dialect::Greybel, source, &["l", "x", "n"]);
    let unknown: Vec<u32> = Analyzer::new(&workspace)
        .diagnostics(&uri(PATH))
        .into_iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::UnknownMember)
        .map(|diagnostic| diagnostic.range.start.line)
        .collect();

    assert_eq!(types, ["list<number or string>", "string", "number"]);
    assert_eq!(unknown, [8]);
}

#[test]
fn should_narrow_by_the_condition_of_an_assert() {
    let source = "x = null\nif rnd then x = \"s\"\nassert x != null, \"x\"\ny = x";

    let types = globals(Dialect::Greybel, source, &["y"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_type_arrow_functions_like_functions() {
    let source = "double = (x) => x * 2\nn = double(2)\nh = (s) =>\n  return s.len\nend function\nm = h(\"ab\")";

    let types = globals(Dialect::Greybel, source, &["n", "m"]);

    assert_eq!(types, ["number", "number"]);
}

#[test]
fn should_type_conditional_expressions_coalescing_and_optional_chains() {
    let source = "x = 1\nif rnd then x = null\na = rnd ? 1 : \"s\"\nb = x ?? \"d\"\nm = {\"n\": {\"k\": 1}}\nif rnd then m = null\nc = m?.n.k\nd = x != null ? x : 0\nf = function(v)\n  return v * 2\nend function\nif rnd then f = null\ne = f?.(2)";

    let types = globals(Dialect::Greybel, source, &["a", "b", "c", "d", "e"]);

    assert_eq!(
        types,
        [
            "number or string",
            "number or string",
            "null or number",
            "number",
            "null or number"
        ]
    );
}

#[test]
fn should_type_the_targets_of_destructuring() {
    let source = "[a, b] = [1, \"s\"]\n{n, p as q, t = 3} = {\"n\": \"x\", \"p\": 2}\n[first, ...others] = [1, 2]";

    let types = globals(Dialect::Greybel, source, &["a", "n", "q", "t", "others"]);

    assert_eq!(
        types,
        [
            "number or string",
            "string",
            "number",
            "number",
            "list<number>"
        ]
    );
}

#[test]
fn should_type_the_names_of_for_headers() {
    let source = "for i, item in [\"a\"]\n  a = i\n  b = item\nend for\nfor [x, y] in [[1, 2]]\n  c = x\nend for\nfor {key, value} in {\"k\": 1}\n  d = key\n  e = value\nend for";

    let types = globals(Dialect::Greybel, source, &["a", "b", "c", "d", "e"]);

    assert_eq!(types, ["number", "string", "number", "string", "number"]);
}

#[test]
fn should_narrow_the_subject_of_switch_cases() {
    let source = "x = 1\nif rnd then x = \"s\"\nswitch x\n  case is string\n    a = x\n  else\n    b = x\nend switch";

    let types = globals(Dialect::Greybel, source, &["a", "b"]);

    assert_eq!(types, ["string", "number"]);
}

#[test]
fn should_narrow_the_subject_of_union_type_cases() {
    let source = "x = 1\nif rnd then x = \"s\"\nif rnd then x = [1]\nswitch x\n  case is string | number\n    a = x\n  else\n    b = x\nend switch";
    let workspace = with_document(Dialect::Greybel, source);

    let types = crate::support::globals_in(&workspace, PATH, &["a", "b"]);
    let diagnostics = Analyzer::new(&workspace).diagnostics(&uri(PATH));

    assert_eq!(types, ["number or string", "list<number>"]);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn should_hover_the_name_and_its_type() {
    let workspace = with_document(
        Dialect::GreyScript,
        "shell = get_shell(\"root\")\ncomputer = shell.host_computer",
    );
    let hover = |line, character| {
        Analyzer::new(&workspace)
            .hover(&uri(PATH), Position::new(line, character))
            .map(|hover| hover.text)
    };

    let variable = hover(1, 2);
    let native = hover(1, 10);
    let literal = hover(1, 21);
    let member = hover(2, 19);
    let nothing = hover(9, 1);

    assert_eq!(variable.as_deref(), Some("shell: null or shell or string"));
    assert_eq!(
        native.as_deref(),
        Some(
            "get_shell: function(user: string = \"\", password: string = \"\"): shell or null or string"
        )
    );
    assert_eq!(literal.as_deref(), Some("string"));
    assert_eq!(
        member.as_deref(),
        Some("host_computer: function(): computer")
    );
    assert_eq!(nothing, None);
}

#[test]
fn should_type_an_added_document() {
    let workspace = with_document(Dialect::MiniScript, "a = 1");

    let a = main_global(&workspace, "a");

    assert_eq!(a, "number");
}

#[test]
fn should_pick_up_an_edit() {
    let mut workspace = with_document(Dialect::MiniScript, "a = 1");

    main_global(&workspace, "a");
    workspace.set_file(&uri(PATH), "a = \"x\"");

    let a = main_global(&workspace, "a");

    assert_eq!(a, "string");
}

#[test]
fn should_remove_an_edited_document() {
    let mut workspace = with_document(Dialect::MiniScript, "a = 1");

    main_global(&workspace, "a");
    workspace.set_file(&uri(PATH), "a = \"x\"");
    main_global(&workspace, "a");

    let removed = workspace.remove_file(&uri(PATH));

    assert!(removed);
}

#[test]
fn should_drop_the_analysis_of_a_removed_document() {
    let mut workspace = with_document(Dialect::MiniScript, "a = 1");

    main_global(&workspace, "a");
    workspace.set_file(&uri(PATH), "a = \"x\"");
    main_global(&workspace, "a");
    workspace.remove_file(&uri(PATH));

    let analysis = Analyzer::new(&workspace).analysis(&uri(PATH));

    assert!(analysis.is_none());
}

#[test]
fn should_reach_the_api_through_members_of_entries() {
    let source = "test = {}
            test.foo = []
            bar = test.foo.hasIndex
            container = {}
            container[0] = {\"value\": 123}
            key = 0
            if rnd then key = \"zero\"
            probe = container[key]
            value = probe.value";

    let types = miniscript(source, &["bar", "probe", "value"]);

    assert_eq!(types, ["number", "map", "number"]);
}

#[test]
fn should_type_isa_and_slices() {
    let source = "test = \"123\" isa string
            foo = [1]
            bar = (foo + foo)[ : ]";

    let types = miniscript(source, &["test", "bar"]);

    assert_eq!(types, ["number", "list<number>"]);
}

#[test]
fn should_fall_back_from_scope_maps_to_map_members() {
    let source = "a = @globals.remove
            b = @remove
            c = globals.missing
            x = 1
            d = globals.x
            e = locals.hasIndex(\"x\")";

    let types = miniscript(source, &["a", "b", "c", "d", "e"]);

    assert_eq!(
        types,
        [
            "function(key: string): number",
            "function(self: list<any> or map<any,any> or string, key: any): list<any> or map<any,any> or string",
            "unknown",
            "number",
            "number"
        ]
    );
}

#[test]
fn should_add_functions_assigned_to_type_maps() {
    let source = "map.test = function(foo=123)
            end function
            list.test = function(foo=123)
            end function
            fn = @map.test
            output = map.test
            literal = @{}.test
            chained = @join.split.test
            called = join.split.test
            map.hasIndex = function(a,b,c)
            end function
            bar = {}
            replaced = @bar.hasIndex
            bar.test = @bar.hasIndex
            stored = @bar.test";

    let types = miniscript(
        source,
        &[
            "fn", "output", "literal", "chained", "called", "replaced", "stored",
        ],
    );

    assert_eq!(
        types,
        [
            "function(foo: number = 123): null",
            "null",
            "function(foo: number = 123): null",
            "function(foo: number = 123): null",
            "null",
            "function(a: any, b: any, c: any): null",
            "function(a: any, b: any, c: any): null"
        ]
    );
}

#[test]
fn should_still_call_what_follows_address_of() {
    let workspace = with_document(
        Dialect::MiniScript,
        "foo = @hasIndex
        a = @hasIndex()
        b = @{}.hasIndex()
        c = @split().join()",
    );

    let analysis = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("added");
    let foo = global(analysis, "foo");
    let called = ["a", "b", "c"].map(|name| global(analysis, name));

    assert_eq!(
        foo,
        "function(self: list<any> or map<any,any> or string, index: any): number or null"
    );
    assert_eq!(called, ["null or number", "number", "string"]);
}

#[test]
fn should_inherit_and_override_along_isa_chains() {
    let source = "test = {\"foo\":123}
            sub = new test
            a = sub.foo
            test2 = new test
            sub2 = new test2
            b = sub2.foo
            test3 = new test
            test3.foo = \"test\"
            sub3 = new test3
            c = sub3.foo
            t = {}
            t.foo = function(a, b, c)
            end function
            bar = @(new t).foo";

    let types = miniscript(source, &["a", "b", "c", "bar"]);

    assert_eq!(
        types,
        [
            "number",
            "number",
            "string",
            "function(a: any, b: any, c: any): null"
        ]
    );
}

#[test]
fn should_end_maps_holding_their_own_class() {
    let mut source = String::from("globals.Test = {\"value\":\"test\"}\n");

    for name in ["A", "B", "C", "D", "E", "F", "G", "Q"] {
        let _ = write!(
            source,
            "Test.{name} = {{\n  \"__isa\": Test,\n  \"subvalue\": \"val\",\n}}\n"
        );
    }

    source.push_str(
        "s = Test.Q.subvalue
        v = Test.Q.value
        Bar = {}
        Bar = new Bar
        Bar.test = {\"moo\":\"null\"}
        Foo = {}
        Foo.__isa = Foo
        Foo.test = function(a, b)
        end function
        nad = new Foo
        missing = nad.doesNotExist
        found = @nad.test",
    );

    let types = globals(Dialect::Greybel, &source, &["s", "v", "missing", "found"]);

    assert_eq!(
        types,
        [
            "string",
            "string",
            "unknown",
            "function(a: any, b: any): null"
        ]
    );
}

#[test]
fn should_let_self_and_super_be_assigned() {
    let source = "MyClass = {}
            CreateClass = function
              self = new MyClass
              self.foo = \"12345\"
              super = new MyClass
              super.bar = 1
              globals.selfFoo = self.foo
              globals.superBar = super.bar
              globals.selfMap = self
            end function";

    let types = miniscript(source, &["selfFoo", "superBar", "selfMap"]);

    assert_eq!(types, ["string", "number", "map"]);
}

#[test]
fn should_make_super_the_parent_of_the_receiver() {
    let source = "test = {}
            foo = new test
            foo.bar = function(a)
              globals.parent = super
            end function";

    let types = miniscript(source, &["parent"]);

    assert_eq!(types, ["map"]);
}

#[test]
fn should_yield_elements_characters_and_pairs_in_for_loops() {
    let source = "foo = \"was\"
            for a in foo
            end for
            l = [1, 2]
            for b in l
            end for
            m = {}
            m.xxx = \"was\"
            for c in m
            end for
            key = c.key
            value = c.value
            x = \"bar\"
            x = [1, 2]
            x = {}
            x.xxx = \"was\"
            for d in x
            end for";

    let types = miniscript(source, &["a", "b", "__b_idx", "key", "value", "d"]);

    assert_eq!(
        types,
        ["string", "number", "number", "string", "string", "map"]
    );
}

#[test]
fn should_hover_an_api_method_with_the_types_of_its_base() {
    let source = "numbers = [1, 2]\nnumbers.push(3)\nscores = {\"a\": 1}\nscores.push(\"b\")\nvalues = scores.values\nadd = @numbers.push";
    let workspace = with_document(Dialect::MiniScript, source);
    let hover = |line, character| {
        Analyzer::new(&workspace)
            .hover(&uri(PATH), Position::new(line, character))
            .map(|hover| hover.text)
    };

    let list_push = hover(2, 10);
    let map_push = hover(4, 9);
    let map_values = hover(5, 18);
    let reference = hover(6, 0);

    assert_eq!(
        list_push.as_deref(),
        Some("push: function(value: number): list<number>")
    );
    assert_eq!(
        map_push.as_deref(),
        Some("push: function(key: string): map {\n  \"a\": number,\n  [string]: number\n}")
    );
    assert_eq!(
        map_values.as_deref(),
        Some("values: function(): list<number>")
    );
    assert_eq!(
        reference.as_deref(),
        Some("add: function(value: number): list<number>")
    );
}

#[test]
fn should_hover_the_content_of_a_map_no_type_declares() {
    let source = r#"point = {"x": 1, "tags": ["a"], 2: {}}
point.label = "p"
make = function
  return {"name": "x"}
end function
Base = {"base": 1}
child = new Base
child.own = 2
counts = {}
counts["k" + 1] = 5
empty = {}
// @type Named
// @property {string} name
Named = {}
// @return {Named}
find = function
end function
named = find"#;
    let workspace = with_document(Dialect::GreyScript, source);
    let hover = |line| {
        Analyzer::new(&workspace)
            .hover(&uri(PATH), Position::new(line, 0))
            .map(|hover| hover.text)
    };

    let literal = hover(1);
    let function = hover(3);
    let instance = hover(7);
    let computed = hover(9);
    let empty = hover(11);
    let declared = hover(18);

    assert_eq!(
        literal.as_deref(),
        Some(
            "point: map {\n  \"x\": number,\n  \"tags\": list<string>,\n  2: map,\n  \"label\": string\n}"
        )
    );
    assert_eq!(
        function.as_deref(),
        Some("make: function(): map {\n  \"name\": string\n}")
    );
    assert_eq!(
        instance.as_deref(),
        Some("child: map {\n  \"own\": number,\n  \"base\": number\n}")
    );
    assert_eq!(
        computed.as_deref(),
        Some("counts: map {\n  [string]: number\n}")
    );
    assert_eq!(empty.as_deref(), Some("empty: map"));
    assert_eq!(declared.as_deref(), Some("named: Named"));
}

#[test]
fn should_read_only_members_every_value_has_on_any() {
    let (source, positions) =
        marked("f = function(c)\n  |v = c.code\n  |w = c.indexes\nend function");
    let workspace = with_document(Dialect::GreyScript, &source);
    let hover = |position| {
        Analyzer::new(&workspace)
            .hover(&uri(PATH), position)
            .map(|hover| hover.text)
    };

    let texts: Vec<Option<String>> = positions.into_iter().map(hover).collect();

    assert_eq!(
        texts,
        [Some("v: any".to_owned()), Some("w: list<any>".to_owned())]
    );
}

#[test]
fn should_return_one_shared_map_from_get_custom_object() {
    let source = "get_custom_object.count = 1
        setName = function
          get_custom_object.name = \"n\"
        end function
        setName
        a = get_custom_object.count
        b = get_custom_object.name
        object = get_custom_object
        c = object.count
        d = get_custom_object.other
        key = \"k\"
        e = get_custom_object[key]";

    let types = greyscript(source, &["a", "b", "c", "d", "e"]);

    assert_eq!(types, ["number", "string", "number", "any", "any"]);
}

#[test]
fn should_type_operations_that_give_null_at_runtime_as_null() {
    let source = "a = 1 - \"x\"
        b = -\"x\"
        c = -null
        d = null + [1]
        e = 1 + null
        f = 1 < \"x\"
        g = 1 | \"x\"
        h = 6 | 3";

    let types = globals(
        Dialect::Greybel,
        source,
        &["a", "b", "c", "d", "e", "f", "g", "h"],
    );

    assert_eq!(
        types,
        [
            "null", "null", "number", "null", "number", "null", "null", "number"
        ]
    );
}

#[test]
fn should_type_the_members_of_classes() {
    let source = "class Animal\n  tags = []\n  constructor(name)\n    self.name = name\n  end function\n  speak()\n    return \"...\"\n  end function\nend class\nclass Dog extends Animal\n  static sound = \"woof\"\nend class\ndog = Dog.create(\"rex\")\np = Point.create\nclass Point\n  x = 1\nend class\ns = Dog.sound\nt = dog.tags\nc = @Dog.constructor\nv = dog.speak";

    let types = globals(Dialect::Greybel, source, &["s", "t", "c", "v", "dog"]);

    assert_eq!(
        types,
        [
            "string",
            "list<unknown>",
            "function(name: any): map",
            "string",
            "map"
        ]
    );
}

#[test]
fn should_type_calls_of_values_with_function_types() {
    let source = "// @param {(value: number) => string} format\n// @return {(number) => list<string>}\napply = function(format)\n  first = format(1)\n  return function(n)\n    return [first]\n  end function\nend function\nmaker = apply(@str)\nr = maker(2)";

    let types = globals(Dialect::Greybel, source, &["apply", "maker", "r"]);

    assert_eq!(
        types,
        [
            "function(format: function(value: number): string): function(arg1: number): list<string>",
            "function(arg1: number): list<string>",
            "list<string>"
        ]
    );
}

#[test]
fn should_read_the_types_written_in_code() {
    let source = "let port: number = 22\nconnect = function(ip: string, retries: number = 3): map|null\n  return null\nend function\nfirst = function<T>(items: list<T>): T\n  return items[0]\nend function\na = first([\"x\"])\nb = get_shell!\nc = [] as list<string>\nf = (x: number) => x";

    let types = globals(
        Dialect::Greybel,
        source,
        &["port", "connect", "first", "a", "b", "c", "f"],
    );

    assert_eq!(
        types,
        [
            "number",
            "function(ip: string, retries: number = 3): map<any,any> or null",
            "function(items: list<T>): T",
            "string",
            "shell or string",
            "list<string>",
            "function(x: number): number"
        ]
    );
}

#[test]
fn should_report_types_written_in_code_and_comments_and_assignments_that_do_not_fit() {
    let source = "let bad: string = 1\n// @param {string} ip\n// @return {number}\nboth = function(ip: number): string\n  return ip\nend function\nodd = function(x: list<)\n  return x\nend function";
    let workspace = with_document(Dialect::Greybel, source);

    let messages = crate::support::workspace::message_texts(&workspace, PATH);

    assert!(
        messages.contains(&"\"bad\" is declared as string but is assigned number.".to_owned()),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"\"ip\" has a type in code and in @param.".to_owned()),
        "{messages:?}"
    );
    assert!(
        messages.contains(&"The return type is in code and in @returns.".to_owned()),
        "{messages:?}"
    );
}

#[test]
fn should_check_literal_types_and_read_them_as_their_base_types() {
    let source = "open = function(protocol: \"tcp\"|\"udp\", port: 22|80): string\n  return protocol + \"!\"\nend function\na = open(\"tcp\", 22)\nb = open(\"tpc\", 23)\nname = \"udp\"\nc = open(name, 80)\npair = function(): [string, number]\nend function\np = pair";
    let types = globals(Dialect::Greybel, source, &["open", "a", "p"]);
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::CallSignatureMismatch);

    assert_eq!(
        types,
        [
            "function(protocol: \"tcp\" or \"udp\", port: 22 or 80): string",
            "string",
            "list<number or string>"
        ]
    );
    assert_eq!(
        reported,
        owned(&[
            (5, "Argument 1 expects \"tcp\" or \"udp\" but got \"tpc\"."),
            (5, "Argument 2 expects 22 or 80 but got 23."),
        ])
    );
}

#[test]
fn should_narrow_the_argument_of_a_type_guard() {
    let source = "isText = function(value): value is string\n  return value isa string\nend function\nw = rnd ? 1 : \"t\"\nif isText(w) then\n  t = w\nelse\n  n = w\nend if";

    let types = globals(Dialect::Greybel, source, &["isText", "t", "n"]);

    assert_eq!(
        types,
        ["function(value: any): value is string", "string", "number"]
    );
}

#[test]
fn should_display_the_guarded_type_like_other_types() {
    let source = "isValue = function(value): value is string | number\n  return value isa string or value isa number\nend function";

    let types = globals(Dialect::Greybel, source, &["isValue"]);

    assert_eq!(types, ["function(value: any): value is number or string"]);
}

#[test]
fn should_report_switch_cases_missing_for_an_enum_type() {
    let source = "enum Protocol\n  Tcp = \"tcp\"\n  Udp = \"udp\"\n  Icmp = \"icmp\"\nend enum\nshow = function(protocol: Protocol)\n  switch protocol\n    case \"tcp\"\n      print 1\n    case Protocol.Udp, \"udp\"\n      print 2\n  end switch\nend function";
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::MissingCase);

    assert_eq!(
        reported,
        owned(&[(7, "The switch has no case for \"icmp\".")])
    );
}

#[test]
fn should_declare_types_and_aliases_with_type_statements() {
    let source = "type Id = string|number\ntype Pair<T> = [T, T]\ntype Repository extends map\n  name: string\n  timeout?: number\n  readonly id: Id\n  find(id: Id): map|null\nend type\nget = function(repo: Repository, key: Id): Pair<string>\n  return [repo.name]\nend function\nlet r: Repository = {}\nq = r.find(1)\nn = r.name\nx = get(r, 1)";

    let types = globals(Dialect::Greybel, source, &["get", "r", "q", "n", "x"]);

    assert_eq!(
        types,
        [
            "function(repo: Repository, key: number or string): list<string>",
            "Repository",
            "map<any,any> or null",
            "string",
            "list<string>"
        ]
    );
}

#[test]
fn should_report_readonly_assignments_and_skip_optional_members_of_implemented_types() {
    let source = "type Options\n  readonly id: number\n  timeout?: number\nend type\nupdate = function(options: Options)\n  options.id = 2\nend function\nclass Config implements Options\n  id = 1\nend class";
    let workspace = with_document(Dialect::Greybel, source);

    let readonly = messages_of_kind(&workspace, DiagnosticKind::ReadonlyMember);
    let implementations = messages_of_kind(&workspace, DiagnosticKind::MissingImplementation);

    assert_eq!(
        readonly,
        owned(&[(6, "\"id\" is readonly in \"Options\".")])
    );
    assert_eq!(implementations, owned(&[]));
}

#[test]
fn should_narrow_unions_of_literal_types_by_equality() {
    let source = "f = function(p: \"tcp\"|\"udp\"|\"icmp\")\n  if p == \"tcp\" then\n    globals.a = p\n  else if p != \"udp\" then\n    globals.b = p\n  else\n    globals.c = p\n  end if\n  if p == \"tcp\" or p == \"udp\" then\n    globals.d = p\n  end if\n  switch p\n    case \"icmp\"\n      globals.e = p\n    else\n      globals.g = p\n  end switch\nend function";

    let types = globals(Dialect::Greybel, source, &["a", "b", "c", "d", "e", "g"]);

    assert_eq!(
        types,
        [
            "\"tcp\"",
            "\"icmp\"",
            "\"udp\"",
            "\"tcp\" or \"udp\"",
            "\"icmp\"",
            "\"tcp\" or \"udp\""
        ]
    );
}

#[test]
fn should_declare_aliases_with_typedef_comments() {
    let source = "// @typedef {string|number} Id\n\n// @typedef {[T, T]} Pair<T>\n\n// @template T\n// @typedef {list<T>|null} Maybe\n\n// @param {Id} key\n// @param {Maybe<number>} values\n// @return {Pair<string>}\nget = function(key, values)\nend function";

    let types = globals(Dialect::Greybel, source, &["get"]);

    assert_eq!(
        types,
        ["function(key: number or string, values: list<number> or null): list<string>"]
    );
}

#[test]
fn should_narrow_literal_members_by_equality() {
    let source = "type Socket extends map\n  protocol: \"tcp\"|\"udp\"\nend type\nf = function(s: Socket)\n  if s.protocol == \"tcp\" then\n    globals.a = s.protocol\n  else\n    globals.b = s.protocol\n  end if\nend function";

    let types = globals(Dialect::Greybel, source, &["a", "b"]);

    assert_eq!(types, ["\"tcp\"", "\"udp\""]);
}
