//! Treats each workspace subfolder containing `.csproj` files as one .NET package whose solution projects
//! ship together. Reads versions from `<Version>` in `Directory.Build.props` or projects, and dependencies
//! from project references and inherited `Directory.Build.props` folder properties.

mod msbuild;
mod package;
mod properties;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use msbuild::MsBuildFile;
use package::PackageFolder;

use super::{FIRST_VERSION, Language, Package, VersionChange};
use crate::edits::EditTable;

/// Elements a version is written into.
const VERSION_ELEMENTS: [&str; 4] = [
    "Version",
    "AssemblyVersion",
    "FileVersion",
    "InformationalVersion",
];

pub struct Dotnet;

impl Language for Dotnet {
    fn name(&self) -> &'static str {
        "dotnet"
    }

    fn prefix(&self) -> &'static str {
        "dotnet"
    }

    fn build_folders(&self) -> &'static [&'static str] {
        &["bin", "obj"]
    }

    /// Finds outermost folders containing `Directory.Build.props` with projects at or below them.
    fn find_workspaces(&self, _root: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
        let projects = files
            .iter()
            .filter(|file| MsBuildFile::is_project(file))
            .collect::<Vec<_>>();
        let mut workspaces: Vec<PathBuf> = Vec::new();

        for props in files.iter().filter(|file| {
            file.file_name()
                .is_some_and(|name| name == MsBuildFile::DIRECTORY_PROPS)
        }) {
            let folder = props.parent().unwrap_or(Path::new("")).to_path_buf();
            let has_packages = projects.iter().any(|project| {
                project.starts_with(&folder)
                    && project.components().count() > folder.components().count() + 1
            });

            if has_packages && !workspaces.iter().any(|other| folder.starts_with(other)) {
                workspaces.push(folder);
            }
        }

        workspaces
    }

    fn packages(&self, root: &Path, workspace: &Path) -> Result<Vec<Package>> {
        PackageFolder::all(root, workspace)
            .into_iter()
            .map(|folder| folder.package(root))
            .collect()
    }

    /// Replaces `previous` in package-file `Version`, `AssemblyVersion`, `FileVersion`, and
    /// `InformationalVersion` elements.
    fn write_version(&self, change: &VersionChange<'_>, edits: &mut EditTable) -> Result<()> {
        let mut written = false;

        for file in PackageFolder::version_files(edits.root(), &change.package.path) {
            let Some(text) = edits.read(&file)? else {
                continue;
            };
            let replacement = MsBuildFile::parse(&text, &file)?.replace_values(
                &VERSION_ELEMENTS,
                change.previous,
                change.version,
            );

            if let Some(replacement) = replacement {
                edits.write(&file, replacement);
                written = true;
            }
        }

        if !written {
            bail!(
                "{}: no <Version>{}</Version> to replace",
                change.package.path.display(),
                change.previous
            );
        }

        Ok(())
    }

    /// `Directory.Build.props` in the package's folder with the first version, importing the one MSBuild
    /// would otherwise find above.
    fn version_file(&self, root: &Path, package: &Package) -> Option<(PathBuf, String)> {
        if package.version.is_some() {
            return None;
        }

        let above = package
            .path
            .parent()
            .and_then(|parent| MsBuildFile::nearest_props(root, parent));
        let import = if above.is_some() {
            "  <Import Project=\"$([MSBuild]::GetPathOfFileAbove('Directory.Build.props', '$(MSBuildThisFileDirectory)../'))\" />\n"
        } else {
            ""
        };

        Some((
            package.path.join(MsBuildFile::DIRECTORY_PROPS),
            format!(
                "<Project>\n{import}  <PropertyGroup>\n    <Version>{FIRST_VERSION}</Version>\n  </PropertyGroup>\n</Project>\n"
            ),
        ))
    }
}
