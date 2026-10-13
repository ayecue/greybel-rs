use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Args;
use greybel_documents_fs_provider::FsProvider;
use greybel_project::{Project, VariableSettings, insert_assignments};

use super::current_folder;
use crate::report::Failure;

/// Values `#envar` reads, from `NAME=value` files and assignments on the command line, as greybel-js reads them,
/// after those of the project's `greybel.json`.
#[derive(Debug, Clone, Default, Args)]
pub struct VariableOptions {
    /// Read files with one `NAME=value` assignment per line; lines starting with `#` are comments.
    #[arg(long = "env-files", value_name = "FILE", num_args = 1..)]
    pub files: Vec<PathBuf>,
    /// Apply these `NAME=value` assignments after all environment files.
    #[arg(long = "env-vars", value_name = "VAR", num_args = 1..)]
    pub assignments: Vec<String>,
}

impl VariableOptions {
    /// Project's `variables`, then those of the files in their order, then the assignments; a later value of a name
    /// replaces an earlier one.
    pub fn read(
        &self,
        project: &Project,
        variables: &VariableSettings,
    ) -> Result<BTreeMap<String, String>, Failure> {
        let mut values = project.read_variables(&current_folder(), variables, FsProvider::text);

        for file in &self.files {
            let content = std::fs::read_to_string(file).map_err(|error| {
                Failure::input(format!(
                    "Can't read the environment variables in {}: {error}",
                    file.display()
                ))
            })?;

            insert_assignments(content.lines(), &mut values);
        }

        insert_assignments(self.assignments.iter().map(String::as_str), &mut values);

        Ok(values)
    }
}
