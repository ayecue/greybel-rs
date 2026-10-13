//! Set `UPDATE_CS=1` when running `cargo test -p duplex-core --test main cs::` to regenerate the C# files in `duplex/dotnet/duplex-core`.

use std::path::Path;

use duplex_cs::Module;
use duplex_test_support::conformance::definition;
use duplex_test_support::generated::major;

use crate::support::cs::csharp;

#[test]
fn should_generate_the_core_definition_for_the_cs_core() {
    let major = major(Path::new(env!("DUPLEX")).join("core"), 1);

    let module = Module::types(major, "Duplex");

    csharp().assert_up_to_date("src/Definition.g.cs", &module);
}

#[test]
fn should_generate_the_calculator_for_the_cs_tests() {
    let major = major(definition("calculator"), 1);

    let module = Module::protocol(major, "Duplex.Tests.Calculator");

    csharp().assert_up_to_date("tests/Generated/Calculator.g.cs", &module);
}

#[test]
fn should_generate_the_layouts_for_the_cs_tests() {
    let major = major(definition("layouts"), 1);

    let module = Module::types(major, "Duplex.Tests.Layouts");

    csharp().assert_up_to_date("tests/Generated/Layouts.g.cs", &module);
}

#[test]
fn should_generate_the_sensor_for_the_cs_tests() {
    let major = major(definition("tags"), 1);

    let module = Module::protocol(major, "Duplex.Tests.Sensor");

    csharp().assert_up_to_date("tests/Generated/Sensor.g.cs", &module);
}
