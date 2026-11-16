use greybel_core::Dialect;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_source_map::Location;
use greybel_transpiler::{TranspileOptions, Transpiler};

use crate::support::source_map::origin;

#[test]
fn should_map_each_built_line_to_the_file_it_came_from() {
    let mut provider = MemoryProvider::new();

    provider.insert(
        "/project/main.src",
        "#include \"inc.src\"\n#import mod from \"mod.src\"\n\nimport_code(\"lib.src\")\nprint \"main\"\nf = function\n  print \"in f\"\nend function",
    );
    provider.insert("/project/inc.src", "\n\nprint \"inc\"");
    provider.insert(
        "/project/mod.src",
        "x = 1\nmodule.exports = { \"m\": \"mod\" }",
    );
    provider.insert("/project/lib.src", "\nprint \"lib\"");

    let options = TranspileOptions {
        dialect: Dialect::GreyScript,
        ..TranspileOptions::default()
    };
    let outputs = Transpiler::new(&options)
        .transpile_project(&MemoryProvider::uri("/project/main.src"), &provider)
        .unwrap();
    let (main, lib) = (&outputs[0], &outputs[1]);

    assert_eq!(
        origin(main, "\"inc\""),
        Some(Location::new("memory:///project/inc.src", 3))
    );
    assert_eq!(
        origin(main, "\"m\""),
        Some(Location::new("memory:///project/mod.src", 2))
    );
    assert_eq!(
        origin(main, "import_code("),
        Some(Location::new("memory:///project/main.src", 4))
    );
    assert_eq!(
        origin(main, "\"main\""),
        Some(Location::new("memory:///project/main.src", 5))
    );
    assert_eq!(
        origin(main, "\"in f\""),
        Some(Location::new("memory:///project/main.src", 7))
    );
    assert_eq!(origin(main, "MODULES={}"), None);
    assert_eq!(
        origin(lib, "\"lib\""),
        Some(Location::new("memory:///project/lib.src", 2))
    );
}
