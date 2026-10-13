use greybel_transpiler::{TranspileOptions, transpile};

use crate::build::Build;
use crate::error::Result;

/// Transpiles one source without resolving imports. Renaming preserves `params`, the dialect's general API names,
/// and `options.excluded_names`.
///
/// # Errors
///
/// Returns an error when `source` fails to parse.
pub fn transform(source: &str, options: &TranspileOptions) -> Result<String> {
    let options = TranspileOptions {
        excluded_names: Build::excluded_names(options),
        ..options.clone()
    };

    Ok(transpile(source, &options)?)
}
