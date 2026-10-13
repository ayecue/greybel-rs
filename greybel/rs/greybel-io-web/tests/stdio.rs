//! Tests program stdin and stdout against a page endpoint on the same thread.

use std::io;
use std::rc::Rc;

use duplex_core::Driver;
use duplex_test_support::Memory;
use greybel_io::{DEFAULT_INTERVAL, Echo, Key, NullIo, PrintOptions, Screen, Stdin, Stdout};
use greybel_io_web::protocol::{Page, ReadKey, ReadLine};
use greybel_io_web::{PageScreen, PageStdin, PageStdout};
use greybel_test_support::future::ready_output;
use greybel_test_support::io::{Recorder, line, print};
use greybel_test_support::terminal::TerminalRecorder;

#[test]
fn should_send_output_batches_to_the_page() {
    let (page, program, queue) = Memory::connected_pair::<Page>();
    let terminal = TerminalRecorder::serve(&page);
    let batch = [line("hello")];

    PageScreen::new(&program).show(&batch);
    queue.deliver();
    assert_eq!(*terminal.batches.borrow(), [batch.to_vec()]);
}

#[test]
fn should_batch_output_by_the_driver_clock() {
    let (page, program, queue) = Memory::connected_pair::<Page>();
    let terminal = TerminalRecorder::serve(&page);
    let stdout = PageStdout::new(
        PageScreen::new(&program),
        program.driver().scheduler().clone(),
    );

    stdout.print("a", PrintOptions::default());
    stdout.print("b", PrintOptions::default());
    queue.advance(DEFAULT_INTERVAL);
    stdout.print("c", PrintOptions::default());
    queue.deliver();

    let expected = [
        vec![print("a", true, false)],
        vec![print("b", true, false), print("c", true, false)],
    ];

    assert_eq!(*terminal.batches.borrow(), expected);
}

#[test]
fn should_read_lines_through_the_page() {
    let (page, program, queue) = Memory::connected_pair::<Page>();

    page.handle(|read: ReadLine<'static>, response, _next| {
        response.respond(
            if read.echo == Echo::Masked {
                "secret"
            } else {
                "name"
            }
            .to_owned(),
        );
    });
    page.handle(|_: ReadKey<'static>, response, _next| response.respond(Key::Up));

    let stdin = PageStdin::new(&program, Rc::new(NullIo));
    let line = stdin.read_line("password: ", Echo::Masked);
    let _key = stdin.read_key("");

    queue.deliver();

    let line = ready_output(line).ok();

    assert_eq!(line, Some("secret".to_owned()));
}

#[test]
fn should_read_keys_through_the_page() {
    let (page, program, queue) = Memory::connected_pair::<Page>();

    page.handle(|read: ReadLine<'static>, response, _next| {
        response.respond(
            if read.echo == Echo::Masked {
                "secret"
            } else {
                "name"
            }
            .to_owned(),
        );
    });
    page.handle(|_: ReadKey<'static>, response, _next| response.respond(Key::Up));

    let stdin = PageStdin::new(&program, Rc::new(NullIo));
    let _line = stdin.read_line("password: ", Echo::Masked);
    let key = stdin.read_key("");

    queue.deliver();

    let key = ready_output(key).ok();

    assert_eq!(key, Some(Key::Up));
}

#[test]
fn should_interrupt_an_input_the_page_fails() {
    let (page, program, queue) = Memory::connected_pair::<Page>();

    TerminalRecorder::serve(&page);

    let key = PageStdin::new(&program, Rc::new(NullIo)).read_key("");

    queue.deliver();

    let result = ready_output(key);
    let error = result.expect_err("cancelled");

    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert_eq!(error.to_string(), "the input was cancelled");
}

#[test]
fn should_show_the_pending_output_before_an_input() {
    let (_page, program, queue) = Memory::connected_pair::<Page>();
    let stdout = Rc::new(Recorder::default());
    let stdin = PageStdin::new(&program, Rc::clone(&stdout) as Rc<dyn Stdout>);
    let _line = stdin.read_line("", Echo::Visible);

    assert_eq!(stdout.calls(), ["flush"]);
    assert_eq!(queue.queued_batches().len(), 1);
}
