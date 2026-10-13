use crate::dialect::{Dialect, Version};
use crate::lexer::{Lexer, PendingToken, TokenStream};

pub(super) fn block_comment(source: &str) -> Result<String, String> {
    let lexer = Lexer::new(source, Dialect::MiniScript, Version::Latest);
    let mut stream = TokenStream::new();
    let pending = PendingToken::new(0, false);

    lexer
        .scan_block_comment(&mut stream, pending)
        .map(|token| {
            token
                .as_comment()
                .map(|c| c.text(source).to_owned())
                .unwrap_or_default()
        })
        .map_err(|error| error.message())
}
