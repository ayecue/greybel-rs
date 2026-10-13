//! Custom-type fixtures with documented functions and properties.

use greybel_core::Position;
use greybel_type_analyzer::{Database, Dialect, Workspace};

use super::{marked, with_document};

/// Virtual type with a property and documented function, with markers inside a member access and a call.
pub fn greeter() -> (Workspace<Database>, Vec<Position>) {
    let (source, positions) = marked(
        r#"// @vtype Greeter
        // @function greet
        //   @param {string} name - who to greet
        //   @returns {string}
        //   @description Says hello.
        // @property {number} count

        // @define {Greeter}
        g = {}
        msg = g.greet("world")
        g.|greet(|"#,
    );

    (with_document(Dialect::MiniScript, &source), positions)
}

/// Instance of a custom type inheriting another, marked after a member-access dot.
pub fn instance_of_custom_type() -> (Workspace<Database>, Vec<Position>) {
    let (source, at) = marked(
        "// @type Bar
        // @property {string} virtualMoo
        Bar = {}
        Bar.moo = \"\"
        // @return {number}
        Bar.test = function(functionName)
          return 1
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
        test.|",
    );

    (with_document(Dialect::Greybel, &source), at)
}
