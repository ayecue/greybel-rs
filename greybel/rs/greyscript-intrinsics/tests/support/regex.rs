use greybel_interpreter::InterpreterOptions;
use greybel_test_support::interpreter::{constant_clock, interpreter_with_api, run_to_end};
use greyscript_intrinsics::API;
use greyscript_intrinsics::regex::HostRegexEngine;

/// Splits, matches and replaces with patterns the engines read differently.
pub const REGEX_CALLS: &str = r#"
        print "😀".split("[^x]").len
        print "a**".is_match("a**")
        print "Hello".replace("(?<x>e)(l)", "$1$2")
    "#;

/// What `code` prints, on `regex` when given, otherwise the default engine.
pub fn run(regex: Option<HostRegexEngine>, code: &str) -> Vec<String> {
    let (mut interpreter, output) =
        interpreter_with_api(code, InterpreterOptions::new(constant_clock()), API);

    if let Some(regex) = regex {
        interpreter = interpreter.with_host_data(regex);
    }

    run_to_end(&mut interpreter).expect("runs");

    output.take()
}
