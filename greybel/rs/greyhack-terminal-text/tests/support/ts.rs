//! Checks generated TypeScript in `greybel/ts/greyhack-terminal-text`.

use std::path::Path;

use duplex_test_support::generated::GeneratedFolder;

pub fn terminal_text() -> GeneratedFolder {
    GeneratedFolder::typescript(
        Path::new(env!("GREYBEL_TS")).join("greyhack-terminal-text"),
        "greyhack-terminal-text",
    )
}
