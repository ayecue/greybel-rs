use crate::output::BuildOutput;
use crate::scripts::{AUTO_COMPILE, FOLDERS, GreyScriptText};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentFile {
    pub path: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deployment {
    pub directory: String,
    pub resources: String,
    pub auto_compile: bool,
    pub files: Vec<DeploymentFile>,
    pub pre_script: String,
    pub post_script: Option<String>,
}

impl Deployment {
    #[must_use]
    pub fn new(output: &BuildOutput) -> Self {
        let paths: Vec<String> = output
            .files
            .iter()
            .map(|file| format!("/{}", file.path))
            .collect();
        let pre_script = FOLDERS.render(&[(
            "FILE_PATHS",
            &GreyScriptText::list(paths.iter().map(String::as_str)),
        )]);
        let post_script = output.auto_compile.map(|auto_compile| {
            let roots: Vec<String> = output
                .entries
                .iter()
                .map(|path| format!("/{path}"))
                .collect();

            AUTO_COMPILE.render(&[
                (
                    "ROOT_FILE_PATHS",
                    &GreyScriptText::list(roots.iter().map(String::as_str)),
                ),
                (
                    "TEMPORARY",
                    &GreyScriptText::quote(&output.destination.temporary_folder_name),
                ),
                (
                    "ALLOW_IMPORT",
                    if auto_compile.allow_import { "1" } else { "0" },
                ),
            ])
        });

        Self {
            directory: output.destination.directory.clone(),
            resources: output.destination.resources.clone(),
            auto_compile: output.auto_compile.is_some(),
            files: output
                .files
                .iter()
                .zip(paths)
                .map(|(file, path)| DeploymentFile {
                    path,
                    code: file.code.clone(),
                })
                .collect(),
            pre_script,
            post_script,
        }
    }
}
