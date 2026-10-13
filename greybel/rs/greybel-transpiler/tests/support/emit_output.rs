use greybel_transpiler::emit::*;

/// Creates output with two completed lines, comments `x` and `y` on the first, and a trailing unfinished line.
pub fn filled() -> EmitOutput {
    let mut output = EmitOutput::default();

    output.push("a");
    output.eol();
    output.push("b\nc");
    output.eol();
    output.push("unfinished");
    output.append_comment(0, "x");
    output.append_comment(0, "y");

    output
}
