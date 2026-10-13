//! Paths for generated TypeScript modules in `greybel/ts/greybel-api-reference-web`.

use std::path::Path;

use duplex_test_support::generated::GeneratedFolder;

/// Import path used by generated workbench role modules for the workbench protocol.
pub const WORKBENCH: &str = "./workbench.generated.js";

pub fn api_reference_web() -> GeneratedFolder {
    GeneratedFolder::typescript(
        Path::new(env!("GREYBEL_TS")).join("greybel-api-reference-web"),
        "greybel-api-reference",
    )
}
