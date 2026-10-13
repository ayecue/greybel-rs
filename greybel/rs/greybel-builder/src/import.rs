//! Imports editor files into the game unchanged.

use greybel_bundler::{BuildOutput, Destination, OutputFile};
use greybel_documents::{AsyncResourceProvider, PathResolution};

use crate::build::uri;
use crate::deploy::deploy;
use crate::protocol::{BuildFailure, Import};

/// Reads `message`'s files from the provider returned by `files`, deploys them unchanged, and returns their count.
pub async fn import<P: AsyncResourceProvider>(
    message: &Import<'_>,
    files: impl FnOnce(PathResolution) -> P,
) -> Result<u32, BuildFailure> {
    let root = uri(&message.root)?;
    let provider = files(PathResolution::new(Vec::<String>::new()));
    let mut files = Vec::new();

    for file in message.files.iter() {
        let source = uri(file)?;
        let code =
            provider.read(&source).await.ok().flatten().ok_or_else(|| {
                BuildFailure::invalid(format!("{} can't be read", source.label()))
            })?;
        let path = PathResolution::path_in(&root, &source)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| {
                BuildFailure::invalid(format!("{} isn't in {}", source.label(), root.label()))
            })?
            .to_owned();

        files.push(OutputFile { source, path, code });
    }

    let count = u32::try_from(files.len()).unwrap_or(u32::MAX);
    let output = BuildOutput {
        root,
        files,
        entries: Vec::new(),
        destination: Destination::new(&message.ingame_directory, &mut fastrand::Rng::new()),
        auto_compile: None,
    };

    deploy(message.port, &output).await?;

    Ok(count)
}
