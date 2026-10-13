//! Single-document inference across types, members and calls.

use greybel_core::Position;
use greybel_type_analyzer::{Analyzer, Dialect};

use crate::support::infer::FUNCTION_LOCALS;
use crate::support::{
    PATH, global, globals, greyscript, locals, main_global, miniscript, uri, with_document,
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
    let member = "test = {}\ntest.foo = []\nbar = test.foo.hasIndex";

    let once = miniscript(once, &["bar"]);
    let twice = miniscript(twice, &["bar"]);
    let member = miniscript(member, &["bar"]);

    assert_eq!(once, ["number"]);
    assert_eq!(twice, ["list<unknown> or number"]);
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

    assert_eq!(locals, ["number or string", "number or string"]);
}

#[test]
fn should_type_a_global_by_what_a_function_returns() {
    let source = FUNCTION_LOCALS;

    let result = miniscript(source, &["result"]);

    assert_eq!(result, ["number or string"]);
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

    assert_eq!(
        types,
        [
            "number",
            "list<list<list<list<list<list<any>>>>>>",
            "unknown"
        ]
    );
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
            key = \"zero\"
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
        source.push_str(&format!(
            "Test.{name} = {{\n  \"__isa\": Test,\n  \"subvalue\": \"val\",\n}}\n"
        ));
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
        [
            "string",
            "number",
            "number",
            "string",
            "string",
            "map or number or string"
        ]
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
