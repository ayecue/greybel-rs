//! Project build and standalone source-transformation operations.

use greybel_bundler::{BuildError, BuildOptions, Installer};
use greybel_documents::{AsyncResourceProvider, PathResolution, Uri};
use greybel_transpiler::{TranspileError, TranspileOptions};

use crate::deploy::deploy;
use crate::protocol::{Build, BuildFailure, BuildResult, BuiltFile, SourceLocation, Transform};

pub async fn build<P: AsyncResourceProvider>(
    message: &Build<'_>,
    files: impl FnOnce(PathResolution) -> P,
) -> Result<BuildResult, BuildFailure> {
    let entries = message
        .entries
        .iter()
        .map(|entry| uri(entry))
        .collect::<Result<Vec<_>, _>>()?;
    let root = message.root.as_deref().map(uri).transpose()?;
    let options = BuildOptions {
        transpile: TranspileOptions::from(&message.transpile),
        ingame_directory: message.ingame_directory.to_string(),
        output_name: message.output_name.as_ref().map(ToString::to_string),
        auto_compile: message.auto_compile.as_ref().map(|auto_compile| {
            greybel_bundler::AutoCompile {
                allow_import: auto_compile.allow_import,
            }
        }),
    };
    let resolution = PathResolution::new(message.extensions.iter().map(ToString::to_string));
    let provider = files(match entries.first() {
        Some(entry) => resolution.with_base_folder(root, entry),
        None => resolution.with_folders(root),
    });
    let output = match greybel_bundler::Build::new(options, &mut fastrand::Rng::new())
        .run_async(&entries, &provider)
        .await
    {
        Ok(output) => output,
        Err(error) => {
            let location = match &error {
                BuildError::Transpile(TranspileError::Load(error)) => {
                    SourceLocation::of_syntax_error(error, &provider).await
                }
                _ => None,
            };

            return Err(BuildFailure::Invalid {
                message: error.to_string(),
                location,
            });
        }
    };
    let installer = match message.installer_length {
        Some(max_chars) => Installer::new(&output, max_chars as usize)
            .scripts()
            .map_err(|error| BuildFailure::invalid(error.to_string()))?
            .into_iter()
            .map(|script| BuiltFile {
                path: script.name,
                code: script.code,
            })
            .collect(),
        None => Vec::new(),
    };

    if let Some(port) = message.deploy_port {
        deploy(port, &output).await?;
    }

    Ok(BuildResult {
        root: output.root.to_string(),
        files: output
            .files
            .into_iter()
            .map(|file| BuiltFile {
                path: file.path,
                code: file.code,
            })
            .collect(),
        installer,
    })
}

pub fn transform(message: &Transform<'_>) -> Result<String, BuildFailure> {
    greybel_bundler::transform(&message.code, &TranspileOptions::from(&message.transpile))
        .map_err(|error| BuildFailure::invalid(error.to_string()))
}

/// Parses `text` as a document URI or returns `BuildFailure::Invalid` for malformed input.
pub(crate) fn uri(text: &str) -> Result<Uri, BuildFailure> {
    Uri::parse(text).map_err(|error| BuildFailure::invalid(error.to_string()))
}
