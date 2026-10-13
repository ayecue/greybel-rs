use std::path::PathBuf;

use clap::Args;
use greybel_builder::protocol::Import;
use greybel_documents_fs_provider::FsProvider;

use super::block_on;
use super::options::{MessageHook, ProjectOptions};
use crate::report::{Failure, Report};

#[derive(Debug, Args)]
pub struct ImportCommand {
    #[command(flatten)]
    project: ProjectOptions,
    /// File, or the folder of files, to import.
    targetpath: PathBuf,
    /// Folder in the game the files go to [default: greybel.json's build folder in the game, /root/].
    #[arg(long = "ingame-directory")]
    ingame_directory: Option<String>,
    #[command(flatten)]
    message_hook: MessageHook,
    #[command(flatten)]
    report: Report,
}

impl ImportCommand {
    pub fn run(self) -> Result<(), Failure> {
        let files = FsProvider::files(&self.targetpath);

        if files.is_empty() {
            return Err(Failure::input(format!(
                "No files in {}.",
                self.targetpath.display()
            )));
        }

        let target = std::path::absolute(&self.targetpath).map_err(Failure::input)?;
        let ingame_directory = match &self.ingame_directory {
            Some(directory) => directory.clone(),
            None => {
                self.project
                    .project_of(&files[0])?
                    .manifest
                    .build
                    .ingame_directory
            }
        };
        let message = Import {
            root: FsProvider::uri(target.parent().unwrap_or(&target))
                .to_string()
                .into(),
            files: files
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .into(),
            ingame_directory: ingame_directory.as_str().into(),
            port: self.message_hook.port,
        };
        let count = block_on(greybel_builder::import(&message, FsProvider::new))
            .map_err(|failure| Failure::build(&failure))?;

        self.report.progress(format!(
            "Import done. {count} files available in {ingame_directory}."
        ));

        Ok(())
    }
}
