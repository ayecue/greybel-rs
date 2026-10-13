//! Tests that a program's output reaches its terminal in the game's batches.

use crate::support::scripts::run_code_in;

#[test]
fn should_replace_the_terminal_with_a_replacing_print_and_the_prints_after_it() {
    let code = "print \"one\"\nprint(\"two\", true)\nprint \"three\"\nprint(\"four\", true)\nprint \"five\"";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "one\n(clear)\ntwo\nthree\nfour\nfive");
}

#[test]
fn should_send_the_batch_when_the_program_waits() {
    let code =
        "print(\"one\", true)\nprint \"two\"\nwait(0.1)\nprint(\"three\", true)\nprint \"four\"";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "(clear)\none\ntwo\n(clear)\nthree\nfour");
}

#[test]
fn should_drop_what_is_not_sent_when_the_screen_is_cleared() {
    let code = "print \"one\"\nclear_screen\nprint \"two\"";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "(clear)\ntwo");
}

#[test]
fn should_print_a_launched_program_into_its_launchers_batch() {
    let code = "sh = get_shell; pc = sh.host_computer\npc.touch(home_dir, \"child.src\"); pc.File(home_dir + \"/child.src\").set_content(\"print 2\")\nsh.build(home_dir + \"/child.src\", home_dir)\nprint(1, true)\nsh.launch(home_dir + \"/child\")\nprint 3";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "(clear)\n1\n2\n3");
}
