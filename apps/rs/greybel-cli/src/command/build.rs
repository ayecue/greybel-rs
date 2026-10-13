use std::path::{Path, PathBuf};

use clap::Args;
use greybel_builder::protocol::BuiltFile;
use greybel_documents::Uri;
use greybel_documents_fs_provider::FsProvider;
use greybel_project::Project;

use super::block_on;
use super::options::{
    MessageHook, ProjectOptions, SourceOptions, Transpilation, VariableOptions, current_folder,
};
use crate::report::{Failure, Report};

/// Glob pattern metacharacters recognized within a path segment.
const GLOB_CHARACTERS: &[char] = &['*', '?', '[', '{'];

#[derive(Debug, Args)]
pub struct BuildCommand {
    #[command(flatten)]
    project: ProjectOptions,
    /// File to build, or a glob pattern matching the files to build [default: greybel.json's entries].
    filepath: Option<String>,
    /// Folder the build goes to [default: greybel.json's folder, or the current one].
    output: Option<PathBuf>,
    #[command(flatten)]
    sources: SourceOptions,
    #[command(flatten)]
    variables: VariableOptions,
    #[command(flatten)]
    transpilation: Transpilation,
    /// Folder in the game the build runs from [default: /root/].
    #[arg(long = "ingame-directory")]
    ingame_directory: Option<String>,
    /// Generate installer files that recreate builds containing at least one `import_code` in the game.
    #[arg(short = 'i', long)]
    installer: bool,
    /// Do not generate installer files.
    #[arg(long = "no-installer", conflicts_with = "installer")]
    no_installer: bool,
    /// Maximum characters per installer file; longer installers are split [default: 160000].
    #[arg(long = "max-chars")]
    max_chars: Option<u32>,
    /// Compile the build in the game and delete its files after compilation.
    #[arg(long = "auto-compile")]
    auto_compile: bool,
    /// Do not compile the build in the game.
    #[arg(long = "no-auto-compile", conflicts_with = "auto_compile")]
    no_auto_compile: bool,
    /// Allow the compiled program to import code at runtime during in-game compilation.
    #[arg(long = "allow-import")]
    allow_import: bool,
    /// Create the build in Grey Hack using its message hook.
    #[arg(long = "create-ingame")]
    create_ingame: bool,
    #[command(flatten)]
    message_hook: MessageHook,
    /// Write build files directly into the output folder instead of greybel.json's build output (`build`) subfolder.
    #[arg(long = "disable-build-folder")]
    disable_build_folder: bool,
    /// Name of the built entry file, for a build of one file.
    #[arg(
        long = "output-filename",
        alias = "outputFilename",
        value_name = "NAME"
    )]
    output_filename: Option<String>,
    #[command(flatten)]
    report: Report,
}

impl BuildCommand {
    pub fn run(self) -> Result<(), Failure> {
        let (project, entries) = self.entries()?;

        if entries.is_empty() {
            return Err(Failure::input(match &self.filepath {
                Some(pattern) => format!("No files match {pattern}."),
                None => "Name the files to build, or the entries of greybel.json.".to_owned(),
            }));
        }

        let project = self.project(project);

        if project.manifest.build.output_name.is_some() && entries.len() > 1 {
            return Err(Failure::input(
                "The output filename names the build of one file only.",
            ));
        }

        let variables = self
            .variables
            .read(&project, &project.manifest.build.variables)?;
        let message = project.build(
            &entries,
            variables,
            self.create_ingame.then_some(self.message_hook.port),
        );
        let result = block_on(greybel_builder::build(&message, FsProvider::new))
            .map_err(|failure| Failure::build(&failure))?;
        let output = self.output_folder(&project)?;

        write(&output, &result.files)?;
        write(&output, &result.installer)?;

        if self.create_ingame {
            self.report.progress(format!(
                "Created the build in Grey Hack in {}.",
                project.manifest.build.ingame_directory
            ));
        }

        self.report
            .progress(format!("Build done. Available in {}.", output.display()));

        Ok(())
    }

    /// Files to build and the project they are in: those `filepath` names in the project of the first, or the
    /// entries of the current folder's project.
    fn entries(&self) -> Result<(Project, Vec<Uri>), Failure> {
        let Some(pattern) = &self.filepath else {
            let project = self.project.current_project()?;
            let entries = project.entries(&current_folder());

            return Ok((project, entries));
        };
        let entries = entries(pattern);
        let project = match entries.first() {
            Some(entry) => self.project.project_of(entry)?,
            None => Project::default(),
        };

        Ok((project, entries))
    }

    /// Apply option overrides to `project` and return it.
    fn project(&self, mut project: Project) -> Project {
        let manifest = &mut project.manifest;

        self.sources.apply(manifest);
        self.transpilation.apply(manifest);

        let build = &mut manifest.build;

        if let Some(directory) = &self.ingame_directory {
            build.ingame_directory.clone_from(directory);
        }

        if self.installer || self.no_installer {
            build.installer.active = self.installer;
        }

        if let Some(max_chars) = self.max_chars {
            build.installer.max_chars = max_chars;
        }

        if self.auto_compile || self.no_auto_compile {
            build.auto_compile = self.auto_compile;
        }

        if self.allow_import {
            build.installer.allow_import = true;
            build.deploy.allow_import = true;
        }

        if self.output_filename.is_some() {
            build.output_name.clone_from(&self.output_filename);
        }

        project
    }

    /// Folder the build is written into: greybel.json's build output in the output folder, emptied first, or the
    /// output folder itself.
    fn output_folder(&self, project: &Project) -> Result<PathBuf, Failure> {
        let folder = match &self.output {
            Some(output) => output.clone(),
            None => project
                .root
                .as_ref()
                .and_then(FsProvider::path)
                .unwrap_or_else(|| PathBuf::from(".")),
        };

        if self.disable_build_folder {
            return Ok(folder);
        }

        let output = folder.join(&project.manifest.build.output);

        if output.exists() {
            std::fs::remove_dir_all(&output).map_err(|error| {
                Failure::new(
                    "Output error",
                    format!("Can't replace {}: {error}", output.display()),
                )
            })?;
        }

        Ok(output)
    }
}

/// File `pattern` names, or the files it matches when a segment of it is a glob pattern, sorted.
fn entries(pattern: &str) -> Vec<Uri> {
    let path = Path::new(pattern);
    let glob_start = path.components().position(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .contains(GLOB_CHARACTERS)
    });
    let Some(glob_start) = glob_start else {
        return if path.is_file() {
            vec![FsProvider::uri(path)]
        } else {
            Vec::new()
        };
    };
    let folder: PathBuf = path.components().take(glob_start).collect();
    let glob = path
        .components()
        .skip(glob_start)
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");

    FsProvider::find_in(&folder, &glob)
}

/// Write each built file to `output` using its build-relative path.
fn write(output: &Path, files: &[BuiltFile]) -> Result<(), Failure> {
    for file in files {
        let path = output.join(&file.path);

        path.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, &file.code))
            .map_err(|error| {
                Failure::new(
                    "Output error",
                    format!("Can't write {}: {error}", path.display()),
                )
            })?;
    }

    Ok(())
}
