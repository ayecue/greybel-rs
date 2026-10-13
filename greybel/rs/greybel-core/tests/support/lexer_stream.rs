use greybel_core::lexer::TokenSnapshot;
use greybel_core::{Lexer, TokenStream};

pub const RESTORE_SOURCE: &str = "a = .5. b c d";

pub fn values(lexer: &Lexer<'_>, stream: &mut TokenStream, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            lexer.advance(stream).unwrap();

            stream.current().value(lexer.source()).into_owned()
        })
        .collect()
}

pub fn past_snapshot(lexer: &Lexer<'_>) -> (TokenStream, TokenSnapshot, usize, Vec<String>) {
    let mut stream = TokenStream::new_unsafe();

    values(lexer, &mut stream, 2);

    let snapshot = stream.snapshot();
    let errors = stream.errors().len();
    let ahead = values(lexer, &mut stream, 4);

    (stream, snapshot, errors, ahead)
}
