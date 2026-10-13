use super::*;

#[test]
fn should_go_on_with_unclosed_blocks_and_expressions() {
    for code in [
        "f = function(x)",
        "if x then",
        "x = 1 +",
        "while true\nprint(1)",
    ] {
        let error = greybel_core::parse(code, ParserOptions::new(Dialect::GreyScript)).err();

        assert!(
            is_unfinished(code, &ParserOptions::new(Dialect::GreyScript)),
            "{code:?}: {error:?}"
        );
    }
}

#[test]
fn should_run_finished_code_and_code_with_errors_before_its_end() {
    for code in [
        "x = 1",
        "f = function(x)\nreturn x\nend function",
        "end if",
        "x = ) + 1",
    ] {
        assert!(
            !is_unfinished(code, &ParserOptions::new(Dialect::GreyScript)),
            "{code:?}"
        );
    }
}
