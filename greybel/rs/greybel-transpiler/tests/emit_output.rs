use greybel_transpiler::emit::*;

use crate::support::emit_output::filled;

#[test]
fn should_give_no_code_when_empty() {
    let output = EmitOutput::default();
    let code = output.into_code();

    assert_eq!(code, "");
}

#[test]
fn should_keep_finished_lines_with_their_comments() {
    let output = filled();
    let lines = output.lines().collect::<Vec<_>>();

    assert_eq!(lines, [("a", "x y"), ("b\nc", "")]);
}

#[test]
fn should_drop_the_current_line_from_the_code() {
    let output = filled();
    let code = output.into_code();

    assert_eq!(code, "a\nb\nc");
}
