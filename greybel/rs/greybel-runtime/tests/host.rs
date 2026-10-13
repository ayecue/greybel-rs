//! Tests an in-process runtime with its own host and controller.

use greybel_documents_memory_provider::MemoryProvider;
use greybel_runtime::Runtime;
use greybel_runtime::protocol::{Exit, SourceBreakpoint, StopReason};

use crate::support::host::{MAIN, launch, run, setup};

#[test]
fn runs_a_program_with_its_params_and_input() {
    let setup = setup(
        "name = user_input(\"Name: \")\nprint(params[0] + \" \" + name)",
        vec!["Ada"],
    );

    run(async move {
        let (runtime, controller) = Runtime::with_host(setup.host);

        tokio::task::spawn_local(runtime.serve());

        let exit = controller.launch(launch(false, &["Hello"])).await;

        assert!(matches!(exit, Ok(Exit::Completion)), "{exit:?}");
        assert_eq!(
            setup.printed.borrow().last().map(String::as_str),
            Some("Hello Name: Ada")
        );
    });
}

#[test]
fn stops_at_breakpoints_and_inspects_the_program() {
    let mut setup = setup("a = 1\nb = a + 1\nprint(b)", Vec::new());

    run(async move {
        let (runtime, controller) = Runtime::with_host(setup.host);

        tokio::task::spawn_local(runtime.serve());

        let set = controller.set_breakpoints(
            &MemoryProvider::uri(MAIN).to_string(),
            &[SourceBreakpoint {
                line: 2,
                condition: None,
                hit_condition: None,
                log_message: None,
            }],
        );

        assert!(set[0].verified || set[0].message.is_none());

        let exit = controller.launch(launch(true, &[]));
        let exit = tokio::task::spawn_local(exit);
        let stop = setup.stops.recv().await.expect("a stop");

        assert_eq!(stop.reason, StopReason::Breakpoint);
        assert_eq!(
            stop.location.as_ref().map(|location| location.line),
            Some(2)
        );

        let frames = controller.stack_trace().await.expect("a stack");
        let value = controller
            .evaluate("a * 10".to_owned())
            .await
            .expect("an evaluation")
            .expect("a value");

        assert!(!frames.is_empty());
        assert_eq!(value.value, "10");

        controller.resume();

        let exit = exit.await.expect("the launch completes");

        assert!(matches!(exit, Ok(Exit::Completion)), "{exit:?}");
        assert_eq!(setup.printed.borrow().last().map(String::as_str), Some("2"));
        assert!(setup.logs.borrow().is_empty());
    });
}

#[test]
fn terminates_a_running_program() {
    let setup = setup("i = 0\nwhile true\n  i = i + 1\nend while", Vec::new());

    run(async move {
        let (runtime, controller) = Runtime::with_host(setup.host);

        tokio::task::spawn_local(runtime.serve());

        let exit = tokio::task::spawn_local(controller.launch(launch(false, &[])));

        tokio::task::yield_now().await;
        controller.terminate();

        let exit = exit.await.expect("the launch completes");

        assert!(matches!(exit, Ok(Exit::Termination)), "{exit:?}");
    });
}
