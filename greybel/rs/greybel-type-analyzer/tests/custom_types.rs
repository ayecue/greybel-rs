//! Custom types (`@type`, `@vtype`) and generics.

use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect};

use crate::support::custom_types::{greeter, instance_of_custom_type};
use crate::support::features::completion_labels;
use crate::support::workspace::load;
use crate::support::{
    PATH, globals_in, greyscript, locals, main_global, miniscript, uri, with_document,
};

#[test]
fn should_see_the_entries_of_a_returned_custom_type() {
    let source = r#"// @type test
        foo = {}
        foo.xxx = "was"

        // @description This function returns a file!
        // @return {test}
        bar = function
        end function


        abc = bar.xxx"#;

    let types = greyscript(source, &["abc", "foo", "bar"]);

    assert_eq!(types, ["string", "test", "function(): test"]);
}

#[test]
fn should_describe_entries_with_properties() {
    let source = r#"// @type OtherCustomType
        // @property {string} foo
        // @property {number} test
        OtherCustomType = {}

        // @type CustomType
        // @property {list<string>} columns
        CustomType = {}

        item = CustomType.columns[2]

        // @type test
        // @property {number} moo
        foo = {}
        foo.xxx = "was"
        // @return {test}
        bar = function
        end function
        abc = bar.moo"#;

    let types = greyscript(source, &["item", "abc"]);

    assert_eq!(types, ["string", "number"]);
}

#[test]
fn should_describe_a_property_map_with_nested_paths() {
    let source = r#"// @type test
        // @property {map} moo
        // @property {list} moo.bar
        foo = {}
        foo.xxx = "was"

        // @type teq
        teq = new foo

        // @return {test}
        bar = function
        end function
        // @return {teq}
        child = function
        end function

        abc = bar.moo.bar
        inherited = child.moo.bar
        moo = bar.moo
        indexed = bar.moo["bar"]"#;

    let types = greyscript(source, &["abc", "inherited", "teq", "moo", "indexed"]);

    assert_eq!(types, ["list<any>", "list<any>", "teq", "map", "list<any>"]);
}

#[test]
fn should_share_entries_across_declarations_of_a_type() {
    let source = r#"// @type Foo
        foo = {}
        foo.xxx = "hi"

        // @type Foo
        another = {}
        x = another.xxx"#;

    let types = miniscript(source, &["x", "another"]);

    assert_eq!(types, ["string", "Foo"]);
}

#[test]
fn should_reference_and_inherit_custom_types() {
    let source = r#"// @type Outer
        // @property {Inner} child
        outer = {}

        // @type Inner
        // @property {number} value
        inner = {}
        x = outer.child.value

        // @type Parent
        // @property {string} label
        parent = {}

        Child = new parent
        y = Child.label"#;

    let types = miniscript(source, &["x", "y"]);

    assert_eq!(types, ["number", "string"]);
}

#[test]
fn should_keep_the_return_tags_of_custom_type_methods() {
    let source = r#"// @type Bar
        // @property {string} virtualMoo
        Bar = {}
        Bar.moo = ""
        // @return {number}
        Bar.test = function(functionName)
          return self
        end function

        // @type Foo
        Foo = new Bar
        // @return {Foo}
        Foo.New = function(message)
          result = new Foo
          return result
        end function

        test = Foo.New
        abc = test.test
        virtual = test.virtualMoo"#;

    let types = greyscript(source, &["abc", "test", "virtual"]);

    assert_eq!(types, ["number", "Foo", "string"]);
}

#[test]
fn should_share_custom_types_across_files() {
    let provider = MemoryProvider::from_iter([
        (
            "/lib.src",
            r#"// @type test
            foo = {}
            foo.xxx = "was"

            // @return {test}
            bar = function
            end function

            // @vtype myOtherType
            // @property {map<string,string>} moo
            "#,
        ),
        (
            PATH,
            r#"#include "lib.src"
            abc = bar.xxx

            // @vtype myCustomType
            // @extends myOtherType
            // @property {number} myProperty

            // @define {myCustomType}
            baz = {}
            moo = baz.moo"#,
        ),
    ]);
    let workspace = load(Dialect::Greybel, &provider, &[PATH]);

    let types = globals_in(&workspace, PATH, &["abc", "moo"]);

    assert_eq!(types, ["string", "map<string,string>"]);
}

#[test]
fn should_resolve_virtual_types_without_a_map() {
    let source = r#"// @vtype myOtherType
        // @property {map<string,string>} moo

        // @vtype myCustomType
        // @extends myOtherType
        // @function foo
          // @params {string} bar
          // @returns {string}
          // @description This is a custom type with a function
          // @example Example usage of myCustomType
        // @property {number} myProperty

        // @vtype myFooType
        // @property {myCustomType} ct
        // @function make
          // @returns {myCustomType}

        // @define {myCustomType}
        bar = {}
        test = bar.moo
        called = bar.foo("x")
        number = bar.myProperty

        // @define {myFooType}
        foo = {}
        nested = foo.ct.moo
        invoked = foo.make().moo"#;

    let types = miniscript(
        source,
        &["test", "called", "number", "nested", "invoked", "bar"],
    );

    assert_eq!(
        types,
        [
            "map<string,string>",
            "string",
            "number",
            "map<string,string>",
            "map<string,string>",
            "myCustomType"
        ]
    );
}

#[test]
fn should_type_calls_of_declared_functions() {
    let (workspace, _) = greeter();

    let msg = main_global(&workspace, "msg");

    assert_eq!(msg, "string");
}

#[test]
fn should_help_with_the_signature_of_declared_functions() {
    let (workspace, positions) = greeter();

    let help = Analyzer::new(&workspace).signature_help(&uri(PATH), positions[1]);

    let help = help.expect("inside a call");

    assert_eq!(help.label, "greet(name: string): string");
    assert_eq!(
        help.parameters[0].documentation.as_deref(),
        Some("who to greet")
    );
    assert_eq!(
        help.documentation
            .and_then(|doc| doc.description)
            .as_deref(),
        Some("Says hello.")
    );
}

#[test]
fn should_complete_properties_before_declared_functions() {
    let (workspace, positions) = greeter();

    let completions = Analyzer::new(&workspace).completions(&uri(PATH), positions[0]);

    let labels: Vec<&str> = completions.iter().map(|item| item.label.as_str()).collect();

    assert!(labels.starts_with(&["count", "greet"]), "{labels:?}");
}

#[test]
fn should_type_a_variable_by_a_type_tag_naming_an_api_class() {
    let source = "// @type shell\nresult = get_shell\nfoo = result";
    let braced = "// @type {list<string>}\nresult = []";

    let named = greyscript(source, &["result", "foo"]);
    let braced = greyscript(braced, &["result"]);

    assert_eq!(named, ["shell", "shell"]);
    assert_eq!(braced, ["list<string>"]);
}

#[test]
fn should_bind_templates_at_each_call() {
    let source = r#"// @template T
        // @param {T} value
        // @returns {T}
        identity = function(value)
          return value
        end function
        n = identity(1)
        s = identity("text")

        // @param {TItem} value
        // @return {TItem}
        test = function(value)
          temp = value
        end function
        result = test("hello")
        unbound = test

        // @param {list<TItem>} items
        // @param {TItem} fallback
        // @return {TItem}
        first = function(items, fallback)
        end function
        both = first([1], "a")"#;

    let types = miniscript(source, &["n", "s", "result", "unbound", "both"]);

    assert_eq!(
        types,
        ["number", "string", "string", "unknown", "number or string"]
    );
}

#[test]
fn should_type_a_parameter_by_its_template_bound() {
    let source = r#"// @template TItem extends string
        // @param {TItem} value
        // @return {TItem}
        test = function(value)
          size = value.len
        end function"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let types = locals(
        Analyzer::new(&workspace)
            .analysis(&uri(PATH))
            .expect("added"),
        0,
        &["value", "size"],
    );

    assert_eq!(types, ["TItem", "number"]);
}

#[test]
fn should_specialize_generic_custom_types_per_use() {
    let source = r#"// @type Box<T>
        // @property {T} value
        // @property {list<T>} items
        box = {}

        // @define {Box<string>}
        nameBox = box
        label = nameBox.value
        items = nameBox.items

        // @define {Box<number>}
        numberBox = box
        n = numberBox.value

        // @vtype Tree<T>
        // @property {T} value
        // @property {Tree<T>} child

        // @define {Tree<number>}
        tree = {}
        deep = tree.child.child.value

        // @vtype Stack<T>
        // @function pop
        //   @returns {T}
        // @function push
        //   @param {T} item
        //   @returns {Stack<T>}

        // @define {Stack<string>}
        stack = {}
        top = stack.pop
        again = stack.push("a").pop()"#;

    let types = miniscript(
        source,
        &["label", "items", "n", "nameBox", "deep", "top", "again"],
    );

    assert_eq!(
        types,
        [
            "string",
            "list<string>",
            "number",
            "Box<string>",
            "number",
            "string",
            "string"
        ]
    );
}

#[test]
fn should_report_unreadable_tags() {
    let source = "// @param {list<} broken\n// @property {number} orphan\nf = function(broken)\nend function";
    let workspace = with_document(Dialect::MiniScript, source);

    let diagnostics: Vec<(DiagnosticKind, u32)> = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("added")
        .diagnostics(0)
        .iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::InvalidAnnotation)
        .map(|diagnostic| (diagnostic.kind, diagnostic.range.start.line))
        .collect();

    assert_eq!(
        diagnostics,
        [
            (DiagnosticKind::InvalidAnnotation, 1),
            (DiagnosticKind::InvalidAnnotation, 2)
        ]
    );
}

#[test]
fn should_return_any_from_function_properties() {
    let custom = "// @vtype myCustomType
        // @extends map
        // @property {function} MyMethod
        // @define {myCustomType}
        test = null
        x = test.MyMethod";
    let native = "// @vtype crypto
        // @extends map
        // @property {function} MyMethod
        // @define {crypto}
        test = null
        x = test.MyMethod";

    let custom = miniscript(custom, &["x"]);
    let native = greyscript(native, &["test", "x"]);

    assert_eq!(custom, ["any"]);
    assert_eq!(native, ["crypto", "any"]);
}

#[test]
fn should_list_the_entries_of_an_instances_type_as_members() {
    let (workspace, at) = instance_of_custom_type();

    let labels = completion_labels(&workspace, PATH, at[0]);

    assert_eq!(
        labels,
        [
            "New",
            "virtualMoo",
            "moo",
            "test",
            "remove",
            "push",
            "pull",
            "pop",
            "shuffle",
            "sum",
            "hasIndex",
            "indexOf",
            "indexes",
            "len",
            "values",
            "replace"
        ]
    );
}

#[test]
fn should_type_methods_of_an_instance_by_their_return_tags() {
    let (workspace, _) = instance_of_custom_type();

    let abc = main_global(&workspace, "abc");

    assert_eq!(abc, "number");
}
