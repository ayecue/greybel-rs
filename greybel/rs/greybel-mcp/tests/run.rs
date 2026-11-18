//! Tests `run` in the simulated world.

use serde_json::json;

use crate::support::{Client, options, run};

#[test]
fn should_run_a_program_with_its_params_and_input() {
    run(async {
        let mut client = Client::connect(options()).await;

        let (text, failed) = client
            .call(
                "run",
                json!({
                    "code": "name = user_input(\"Name: \")\nprint(\"<b>hi</b> \" + name + \" \" + params[0])",
                    "params": ["p1"],
                    "input": ["Ada"],
                }),
            )
            .await;

        assert!(!failed, "{text}");
        assert_eq!(
            text,
            "The program completed.\n\nTerminal:\nName: Ada\nhi Ada p1\n"
        );
    });
}

#[test]
fn should_report_a_crash_and_terminate_a_program_out_of_time() {
    run(async {
        let mut client = Client::connect(options()).await;

        let (crash, _) = client.call("run", json!({ "code": "x = {}.y" })).await;
        let (timeout, _) = client
            .call(
                "run",
                json!({ "code": "while true\nend while", "timeout_seconds": 0.5 }),
            )
            .await;
        let (input, _) = client
            .call(
                "run",
                json!({ "code": "a = user_input(\"a: \")\nb = user_input(\"b: \")\nprint(a + \"|\" + b)", "input": ["1"] }),
            )
            .await;

        assert!(crash.starts_with("The program failed: "), "{crash}");
        assert!(crash.contains("at code.src:1:"), "{crash}");
        assert!(
            timeout.starts_with("The program was terminated after 0.5 seconds."),
            "{timeout}"
        );
        assert!(input.ends_with("Terminal:\na: 1\nb: 1|\n"), "{input}");
    });
}
