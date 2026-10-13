//! Checks generated TypeScript in `greybel/ts/greyhack-terminal-web`.

use std::path::Path;

use duplex_test_support::generated::GeneratedFolder;

pub fn terminal_web() -> GeneratedFolder {
    GeneratedFolder::typescript(
        Path::new(env!("GREYBEL_TS")).join("greyhack-terminal-web"),
        "greyhack-terminal-web",
    )
}
