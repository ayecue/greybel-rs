//! Tests the hover and go to definition on the path of an import directive.

use greybel_core::Position;
use greybel_type_analyzer::Dialect;

use crate::support::import_path::{MATH, definition_at, hover_at, origin_at};

#[test]
fn should_summarize_an_included_file_with_its_header() {
    let entry = "#include \"lib/math.src\"\nx = add(1, 2)";

    let (text, description) =
        hover_at(Dialect::Greybel, entry, &[MATH], Position::new(1, 14)).expect("a hover");

    assert_eq!(text, "Import statement for lib/math.src");
    assert_eq!(
        description,
        "Contains 1 function, 1 variable.\n\nMath helpers."
    );
}

#[test]
fn should_count_imports_namespaces_functions_and_variables() {
    let entry = "import_code(\"app/init.src\")\n";
    let init = (
        "/app/app/init.src",
        "import_code(\"/app/app/a.src\")\nimport_code(\"/app/app/b.src\")\nApp = {}\nApp.utils = {}\nApp.utils.format = function(x)\n  return x\nend function\nApp[\"name\"] = \"x\"\nApp.name = \"y\"\nk = \"z\"\nApp[k] = 1\n",
    );
    let files = [init, ("/app/app/a.src", ""), ("/app/app/b.src", "")];

    let (text, description) =
        hover_at(Dialect::GreyScript, entry, &files, Position::new(1, 16)).expect("a hover");

    assert_eq!(text, "Import statement for app/init.src");
    assert_eq!(
        description,
        "Contains 2 imports, 2 namespaces, 1 function, 3 variables."
    );
}

#[test]
fn should_count_the_types_a_type_include_declares() {
    let entry = "#include type \"types.src\"\n";
    let types = (
        "/app/types.src",
        "// @type Point\n// @property {number} x\n\n// @vtype Line\n// @property {Point} start\n",
    );

    let (_, description) =
        hover_at(Dialect::Greybel, entry, &[types], Position::new(1, 18)).expect("a hover");

    assert_eq!(description, "Contains 2 types.");
}

#[test]
fn should_say_when_a_file_contains_nothing() {
    let entry = "import_code(\"lib.src\")\n";
    let lib = ("/app/lib.src", "print 1\n");

    let (_, description) =
        hover_at(Dialect::GreyScript, entry, &[lib], Position::new(1, 15)).expect("a hover");

    assert_eq!(description, "Contains no imports, assignments or types.");
}

#[test]
fn should_report_a_missing_file() {
    let entry = "#include \"nope.src\"\n";

    let (text, description) =
        hover_at(Dialect::Greybel, entry, &[], Position::new(1, 12)).expect("a hover");

    assert_eq!(text, "Import statement for nope.src");
    assert_eq!(description, "Cannot find the file \"nope.src\".");
}

#[test]
fn should_keep_the_normal_hover_outside_the_path() {
    let entry = "#import Util from \"util.src\"\nprint Util.level";
    let util = ("/app/util.src", "module.exports = {\"level\": 1}\n");

    let (text, description) =
        hover_at(Dialect::Greybel, entry, &[util], Position::new(1, 10)).expect("a hover");

    assert!(text.starts_with("Util: map"), "{text}");
    assert_eq!(description, "");
}

#[test]
fn should_go_to_the_file_an_import_path_names() {
    let util = ("/app/util.src", "module.exports = {}\n");
    let types = ("/app/types.src", "// @type Point\n");
    let entry = "#include \"lib/math.src\"\n#include type \"types.src\"\n#import Util from \"util.src\"\nimport_code(\"lib/math.src\")";
    let files = [MATH, util, types];
    let at = |line, character| {
        definition_at(
            Dialect::GreyScript,
            entry,
            &files,
            Position::new(line, character),
        )
    };

    assert_eq!(
        at(1, 14),
        [("/app/lib/math.src".to_owned(), Position::new(1, 1))]
    );
    assert_eq!(
        at(2, 18),
        [("/app/types.src".to_owned(), Position::new(1, 1))]
    );
    assert_eq!(
        at(3, 22),
        [("/app/util.src".to_owned(), Position::new(1, 1))]
    );
    assert_eq!(
        at(4, 16),
        [("/app/lib/math.src".to_owned(), Position::new(1, 1))]
    );
}

#[test]
fn should_go_nowhere_for_a_missing_file() {
    let entry = "#include \"nope.src\"\n";

    let locations = definition_at(Dialect::Greybel, entry, &[], Position::new(1, 12));

    assert!(locations.is_empty(), "{locations:?}");
}

#[test]
fn should_start_a_definition_from_the_import_path_or_the_name() {
    let entry = "#include \"lib/math.src\"\nx = add(1, 2)";

    let path = origin_at(Dialect::Greybel, entry, &[MATH], Position::new(1, 14));
    let name = origin_at(Dialect::Greybel, entry, &[MATH], Position::new(2, 6));

    assert_eq!(path, Some((Position::new(1, 11), Position::new(1, 23))));
    assert_eq!(name, Some((Position::new(2, 5), Position::new(2, 8))));
}
