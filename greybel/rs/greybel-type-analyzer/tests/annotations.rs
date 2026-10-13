//! Comment annotations (`@param`, `@return`, `@define`) and their effect on types.

use greybel_core::Position;
use greybel_type_analyzer::{Analyzer, Dialect};

use crate::support::{
    PATH, global, greyscript, locals, main_global, miniscript, uri, with_document,
};

#[test]
fn should_type_a_function_by_param_and_return_tags() {
    let source = "// Hello world
        // I am **bold**
        // @param {string} test - The title of the book.
        // @params {string|number} abc - The author of the book.
        // @param {list<string>|map<string,list<string>>} foo - Foobar.
        // @return {crypto} - Some info about return
        test = function(test, abc, foo)
        end function
        output = test";
    let workspace = with_document(Dialect::GreyScript, source);

    let analysis = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("added");
    let output = global(analysis, "output");
    let parameters = locals(analysis, 0, &["test", "abc", "foo"]);
    let function = analysis.model().functions().next().expect("a function");
    let doc = analysis
        .model()
        .function(function)
        .doc
        .as_ref()
        .expect("documented");

    assert_eq!(output, "crypto");
    assert_eq!(
        parameters,
        [
            "string",
            "number or string",
            "list<string> or map<string,list<string>>"
        ]
    );
    assert_eq!(
        doc.description.as_deref(),
        Some("Hello world\nI am **bold**")
    );
    assert_eq!(
        doc.param("abc")
            .and_then(|param| param.description.as_deref()),
        Some("The author of the book.")
    );
}

#[test]
fn should_let_the_body_decide_without_a_return_tag() {
    let params_only =
        "// Hello world\n// @param {number} a\ntest = function(a)\nend function\noutput = test";
    let unknown_tags = "// @public\n// @description Hello world\n// @example print \"test\"\n// Hello world\ntest = function(a)\nend function\noutput = test";

    let params_only = miniscript(params_only, &["output"]);
    let unknown_tags = miniscript(unknown_tags, &["output"]);

    assert_eq!(params_only, ["null"]);
    assert_eq!(unknown_tags, ["null"]);
}

#[test]
fn should_take_nested_types_in_return_tags() {
    let nested = "// @return {map<string,list<string>>} - Some info\ntest = function\nend function\noutput = test";
    let alternative =
        "// @returns {string}\ntest = function\n  return 1\nend function\noutput = test";

    let nested = miniscript(nested, &["output"]);
    let alternative = miniscript(alternative, &["output"]);

    assert_eq!(nested, ["map<string,list<string>>"]);
    assert_eq!(alternative, ["string"]);
}

#[test]
fn should_override_the_assigned_type_with_define() {
    let list = "// @define {list<string>}\nfoo = 123";
    let unknown = "// @define {unknown}\nfoo = 123";
    let unknown_function = "// @define {unknown}\nfoo = function(bar = 123)\nend function";
    let indexed = "// @define {list<string>}\nresult = []\nchosenIdx = \"1\"\nchosenIdx = 0\nselectedItem = result[chosenIdx]";

    let list = miniscript(list, &["foo"]);
    let unknown = miniscript(unknown, &["foo"]);
    let unknown_function = miniscript(unknown_function, &["foo"]);
    let indexed = miniscript(indexed, &["selectedItem"]);

    assert_eq!(list, ["list<string>"]);
    assert_eq!(unknown, ["number"]);
    assert_eq!(unknown_function, ["function(bar: number = 123): null"]);
    assert_eq!(indexed, ["string"]);
}

#[test]
fn should_see_the_programs_map_additions_on_a_map_parameter() {
    let workspace = with_document(
        Dialect::MiniScript,
        "// @return {string}
        map.bar = function
        end function

        // @param {map} abc
        // @return {number}
        test = function(abc)
          result = abc.bar
        end function",
    );

    let types = locals(
        Analyzer::new(&workspace)
            .analysis(&uri(PATH))
            .expect("added"),
        1,
        &["abc", "result"],
    );

    assert_eq!(types, ["map<any,any>", "string"]);
}

#[test]
fn should_give_nested_functions_their_own_block() {
    let workspace = with_document(
        Dialect::MiniScript,
        "// @return {list<string>}
        foo = function
          // @param {string} a
          // @param {string} b
          // @return {list<string>}
          globals.test = function(a, b)

          end function
        end function",
    );

    let test = main_global(&workspace, "test");

    assert_eq!(test, "function(a: string, b: string): list<string>");
}

#[test]
fn should_name_api_classes_and_program_maps_in_types() {
    let source = "Animal = {\"legs\": 4}
        // @param {Animal} animal
        // @param {shell|null} remote
        count = function(animal, remote)
          return [animal.legs, remote.host_computer]
        end function
        result = count";

    let types = greyscript(source, &["result"]);

    assert_eq!(types, ["list<computer or number>"]);
}

#[test]
fn should_document_only_the_next_code_line() {
    let source = "// @define {string}

        a = 1
        // @define {string}


        b = 1
        c = 1 // @define {string}";

    let types = miniscript(source, &["a", "b", "c"]);

    assert_eq!(types, ["string", "number", "number"]);
}

#[test]
fn should_show_documentation_on_hover() {
    let workspace = with_document(
        Dialect::GreyScript,
        "// Greets.\n// @example greet\ngreet = function\nend function\ngreet\nprint get_shell",
    );
    let documentation = |line, character| {
        Analyzer::new(&workspace)
            .hover(&uri(PATH), Position::new(line, character))
            .and_then(|hover| hover.documentation)
    };

    let greet = documentation(5, 2);
    let native = documentation(6, 8);

    let greet = greet.expect("documented");

    assert_eq!(greet.description.as_deref(), Some("Greets."));
    assert_eq!(greet.examples, ["greet"]);

    let native = native.expect("documented");

    assert!(
        native
            .description
            .is_some_and(|description| !description.is_empty())
    );
}

#[test]
fn should_show_optional_parameters_in_signature_help() {
    let source = "// @param {number} [count] - how many\nf = function(count)\nend function\nf(";
    let workspace = with_document(Dialect::MiniScript, source);

    let help = Analyzer::new(&workspace).signature_help(&uri(PATH), Position::new(4, 3));

    let help = help.expect("inside a call");

    assert_eq!(help.label, "f(count: number?): null");
    assert_eq!(
        help.parameters[0].documentation.as_deref(),
        Some("how many")
    );
}

#[test]
fn should_keep_the_lines_of_descriptions() {
    let workspace = with_document(
        Dialect::MiniScript,
        "// **Hello** world
        test = function()
        end function
        // **Hello** world
        // another line
        foo = function()
        end function
        // **Hello** world
        // another line
        // @return {number}
        bar = function()
        end function",
    );

    let analysis = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("added");
    let description = |name| {
        let ty = analysis.global_type(name).expect("a global");

        analysis.documentation(ty).and_then(|doc| doc.description)
    };
    let test = description("test");
    let others = ["foo", "bar"].map(description);

    assert_eq!(test.as_deref(), Some("**Hello** world"));
    for description in others {
        assert_eq!(
            description.as_deref(),
            Some("**Hello** world\nanother line")
        );
    }
}

#[test]
fn should_see_what_the_program_adds_to_a_param_type() {
    let source = "// @return {string}
            map.bar = function
            end function
            // @param {map} abc
            test = function(abc)
              return abc.bar
            end function
            result = test({})";

    let types = miniscript(source, &["result"]);

    assert_eq!(types, ["string"]);
}
