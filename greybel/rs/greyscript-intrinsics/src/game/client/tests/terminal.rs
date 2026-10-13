use std::time::Duration;

use greybel_interpreter::{PrintOptions, Stdout};

#[path = "support/terminal.rs"]
mod support;

use support::output;

#[test]
fn should_send_a_batch_every_200_ms_the_program_runs() {
    let (output, sent, clock) = output();

    output.print("one", PrintOptions::default());
    clock.advance(Duration::from_millis(150));
    output.print("two", PrintOptions::default());
    clock.advance(Duration::from_millis(100));
    output.flush_if_due();
    output.print("three", PrintOptions::default());
    output.send();

    assert_eq!(*sent.0.borrow(), ["one\ntwo", "three"]);
}

#[test]
fn should_keep_the_batch_on_a_flush() {
    let (output, sent, _) = output();

    output.print("one", PrintOptions::default());
    output.flush();

    assert!(sent.0.borrow().is_empty());
}

#[test]
fn should_keep_the_last_160000_characters_of_a_batch() {
    let (output, sent, _) = output();

    output.print(&"a".repeat(100_000), PrintOptions::default());
    output.print(&"b".repeat(100_000), PrintOptions::default());
    output.send();

    let texts = sent.0.borrow();
    let text = &texts[0];

    assert_eq!(text.len(), 160_000 + 1);
    assert!(text.starts_with(&"a".repeat(60_000)));
}
