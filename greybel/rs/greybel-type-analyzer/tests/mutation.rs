//! Tests mutations through API functions, `@mutates` annotations, and index assignment.

use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect};

use crate::support::{PATH, greyscript, miniscript, uri, with_document};

#[test]
fn should_widen_a_list_on_push() {
    let source = "items = []
        items.push(1)
        items.insert(0, \"a\")
        first = items[0]
        again = items.push(2)
        popped = items.pop
        values = items.values
        empty = []
        nothing = empty.pop";

    let types = miniscript(
        source,
        &[
            "items", "first", "again", "popped", "values", "empty", "nothing",
        ],
    );

    assert_eq!(
        types,
        [
            "list<number or string>",
            "number or string",
            "list<number or string>",
            "null or number or string",
            "list<number or string>",
            "list<unknown>",
            "null"
        ]
    );
}

#[test]
fn should_widen_a_list_on_index_assignment() {
    let source = "xs = []\nxs[0] = \"a\"\nys = [1]\nys[ys.len - 1] = \"b\"";

    let types = miniscript(source, &["xs", "ys"]);

    assert_eq!(types, ["list<string>", "list<number or string>"]);
}

#[test]
fn should_widen_members_and_self() {
    let source = "o = {\"items\": []}
        o.items.push(1)
        n = o.items
        Bag = {\"all\": []}
        Bag.add = function(value)
          self.all.push(value)
        end function
        Bag.add(\"x\")
        all = Bag.all";

    let types = miniscript(source, &["n", "all"]);

    assert_eq!(types, ["list<number>", "list<any>"]);
}

#[test]
fn should_add_keys_when_pushing_onto_a_map() {
    let source = "seen = {}
        seen.push(\"a\")
        keys = seen.indexes
        values = seen.values
        key = seen.pop
        same = seen.push(\"b\")";

    let types = greyscript(source, &["keys", "values", "key", "same"]);

    assert_eq!(
        types,
        ["list<string>", "list<number>", "null or string", "map"]
    );
}

#[test]
fn should_not_widen_aliases() {
    let source = "a = []\nb = a\nb.push(1)";

    let types = miniscript(source, &["a", "b"]);

    assert_eq!(types, ["list<unknown>", "list<number>"]);
}

#[test]
fn should_widen_the_argument_a_mutates_tag_names() {
    let source = "// @param {list<T>} out
        // @param {T} item
        // @mutates out
        add = function(out, item)
          out.push(item)
        end function
        names = []
        add(names, \"x\")
        add(names, 1)";

    let types = miniscript(source, &["names"]);

    assert_eq!(types, ["list<number or string>"]);
}

#[test]
fn should_widen_a_generic_custom_type_by_mutates() {
    let source = "// @vtype Stack<T>
        // @function push
        //   @param {T} item
        //   @returns {self}
        //   @mutates
        // @function pop
        //   @returns {T}

        // @define {Stack<string>}
        stack = {}
        again = stack.push(1)
        top = stack.pop";

    let types = miniscript(source, &["stack", "again", "top"]);

    assert_eq!(
        types,
        [
            "Stack<number or string>",
            "Stack<number or string>",
            "number or string"
        ]
    );
}

#[test]
fn should_report_a_mutates_tag_naming_no_parameter() {
    let source = "// @param {list<T>} out
        // @mutates items
        f = function(out)
        end function";
    let workspace = with_document(Dialect::MiniScript, source);

    let messages: Vec<String> = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("added")
        .diagnostics(0)
        .iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::InvalidAnnotation)
        .map(|diagnostic| diagnostic.message.clone())
        .collect();

    assert_eq!(messages, ["@mutates items names no @param"]);
}
