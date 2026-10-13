use greybel_core::{Dialect, Lexer, TokenStream, Version};

use crate::support::lexer_stream::{RESTORE_SOURCE, past_snapshot, values};

#[test]
fn should_read_the_first_tokens_from_a_new_stream() {
    let lexer = Lexer::new("a = b + c", Dialect::MiniScript, Version::Latest);
    let mut first = TokenStream::new();

    let read = values(&lexer, &mut first, 3);

    assert_eq!(read, ["a", "=", "b"]);
}

#[test]
fn should_start_a_second_stream_at_the_beginning() {
    let lexer = Lexer::new("a = b + c", Dialect::MiniScript, Version::Latest);
    let mut first = TokenStream::new();
    let mut second = TokenStream::new();

    values(&lexer, &mut first, 3);

    let read = values(&lexer, &mut second, 1);

    assert_eq!(read, ["a"]);
}

#[test]
fn should_continue_the_first_stream_where_it_stopped() {
    let lexer = Lexer::new("a = b + c", Dialect::MiniScript, Version::Latest);
    let mut first = TokenStream::new();
    let mut second = TokenStream::new();

    values(&lexer, &mut first, 3);
    values(&lexer, &mut second, 1);

    let read = values(&lexer, &mut first, 2);

    assert_eq!(read, ["+", "c"]);
}

#[test]
fn should_continue_the_second_stream_where_it_stopped() {
    let lexer = Lexer::new("a = b + c", Dialect::MiniScript, Version::Latest);
    let mut first = TokenStream::new();
    let mut second = TokenStream::new();

    values(&lexer, &mut first, 3);
    values(&lexer, &mut second, 1);
    values(&lexer, &mut first, 2);

    let read = values(&lexer, &mut second, 2);

    assert_eq!(read, ["=", "b"]);
}

#[test]
fn should_collect_lexer_errors_past_the_snapshot() {
    let lexer = Lexer::new(RESTORE_SOURCE, Dialect::MiniScript, Version::Latest);
    let mut stream = TokenStream::new_unsafe();

    values(&lexer, &mut stream, 2);

    let errors = stream.errors().len();

    values(&lexer, &mut stream, 4);

    assert!(stream.errors().len() > errors);
}

#[test]
fn should_restore_the_token_and_errors_of_the_snapshot() {
    let lexer = Lexer::new(RESTORE_SOURCE, Dialect::MiniScript, Version::Latest);
    let (mut stream, snapshot, errors, _) = past_snapshot(&lexer);

    stream.restore(snapshot);

    assert_eq!(stream.current().value(lexer.source()), "=");
    assert_eq!(stream.errors().len(), errors);
}

#[test]
fn should_resume_from_a_restored_snapshot() {
    let lexer = Lexer::new(RESTORE_SOURCE, Dialect::MiniScript, Version::Latest);
    let (mut stream, snapshot, _, ahead) = past_snapshot(&lexer);

    stream.restore(snapshot);

    let read = values(&lexer, &mut stream, 4);

    assert_eq!(read, ahead);
}
