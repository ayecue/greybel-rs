//! Validates cross-file consistency, releases shipped project dependencies, and requires pending change files
//! for every project modified since the previous release.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;

use crate::changes::{Level, ReleaseRecord};
use crate::diff::Diff;
use crate::git::Git;
use crate::language::{Scope, VersionSource};
use crate::release::plan::Change;
use crate::repository::{Repository, Via};

/// Maximum changed files named before counting the remainder.
const SAMPLE_FILES: usize = 3;
/// Maximum unowned files named in a report.
const SAMPLE_ORPHANS: usize = 5;

#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub notes: Vec<String>,
}

/// Executes all repository validations once.
pub struct Check<'a> {
    repository: &'a Repository,
    report: Report,
}

impl<'a> Check<'a> {
    /// Report of every validation, change files counted from `base` (default: the last release).
    pub fn run(repository: &'a Repository, base: Option<&str>) -> Result<Report> {
        let mut check = Check {
            repository,
            report: Report::default(),
        };

        check.problems();
        check.shipped_packages();
        check.versions();
        check.apps();
        check.dev_packages();
        check.development_workspaces();
        check.changelogs();
        check.changes(base)?;

        Ok(check.report)
    }

    /// Repository loading diagnostics.
    fn problems(&mut self) {
        for problem in &self.repository.problems {
            let list = if problem.error {
                &mut self.report.errors
            } else {
                &mut self.report.notes
            };

            list.push(problem.message.clone());
        }
    }

    /// Project dependencies and `uses` must reference projects so they cannot change without a
    /// release.
    fn shipped_packages(&mut self) {
        for edge in &self.repository.edges {
            let (from, to) = (
                &self.repository.nodes[edge.from],
                &self.repository.nodes[edge.to],
            );

            if edge.scope != Scope::Normal || !from.is_project() || to.is_project() {
                continue;
            }

            let via = match &edge.via {
                Via::Manifest => "its manifest".to_owned(),
                Via::Reference(file) => file.display().to_string(),
                Via::Uses => "`uses`".to_owned(),
            };

            self.report.errors.push(format!(
                "{} ships {} ({via}), which has no [project]",
                from.id, to.id
            ));
        }
    }

    /// Requires a version for every project, using semver where reman writes it.
    fn versions(&mut self) {
        for (_, node) in self.repository.projects() {
            match &node.package.version {
                None => self
                    .report
                    .errors
                    .push(format!("{} has no version in its manifest", node.id)),
                Some(version)
                    if node.language.version_source() == VersionSource::Manifest
                        && semver::Version::parse(version).is_err() =>
                {
                    self.report
                        .errors
                        .push(format!("{}: `{version}` is not a semver version", node.id));
                }
                Some(_) => {}
            }
        }
    }

    /// Requires unique release tags for apps and allows only one repository-latest release.
    fn apps(&mut self) {
        let mut tags = BTreeMap::<String, Vec<&str>>::new();
        let mut latest = Vec::new();

        for (index, node) in self.repository.projects() {
            if node.is_released() {
                tags.entry(self.repository.tag(index, "{version}"))
                    .or_default()
                    .push(node.id.as_str());
            }

            if node.project.as_ref().is_some_and(|project| project.latest) {
                latest.push(node.id.as_str());
            }
        }

        for (tag, ids) in tags.iter().filter(|(_, ids)| ids.len() > 1) {
            self.report.errors.push(format!(
                "released apps {} share the tag {tag}",
                ids.join(" and ")
            ));
        }

        if latest.len() > 1 {
            self.report.errors.push(format!(
                "only one project can be `latest`: {}",
                latest.join(", ")
            ));
        }
    }

    fn dev_packages(&mut self) {
        let ids = self
            .repository
            .nodes
            .iter()
            .filter(|node| !node.is_project())
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();

        if !ids.is_empty() {
            self.report.notes.push(format!(
                "{} package(s) without a [project], never released: {}",
                ids.len(),
                ids.join(", ")
            ));
        }
    }

    /// Development workspace has no project.
    fn development_workspaces(&mut self) {
        for (_, node) in self.repository.projects() {
            let workspace = &self.repository.workspaces[node.workspace];

            if workspace.development {
                self.report.errors.push(format!(
                    "{} is a [project] in {}, a workspace for development only",
                    node.id,
                    workspace.path.display()
                ));
            }
        }
    }

    /// Requires the changelog written by `reman init` for every project.
    fn changelogs(&mut self) {
        for (_, node) in self.repository.projects() {
            let path = node.changelog();

            if !self.repository.root.join(&path).exists() {
                self.report.errors.push(format!(
                    "{} has no changelog {} (`reman init` writes one)",
                    node.id,
                    path.display()
                ));
            }
        }
    }

    /// Pending change files are valid and name every project changed since `base`.
    fn changes(&mut self, base: Option<&str>) -> Result<()> {
        let folder = self.repository.changes();
        let changes = folder.files().and_then(|files| {
            files
                .iter()
                .map(|file| Change::resolve(self.repository, file))
                .collect()
        });
        let changes: Vec<Change> = match changes {
            Ok(changes) => changes,
            Err(error) => {
                self.report.errors.push(format!("{error:#}"));

                return Ok(());
            }
        };

        self.declared_versions(&changes, folder.record()?.as_ref());

        let diff = Diff::since(self.repository, base)?;

        self.unnamed_projects(&diff, &changes);
        self.orphans(&diff);

        Ok(())
    }

    /// Project whose versions its file names declare is named by the level its version moved since the last
    /// release.
    fn declared_versions(&mut self, changes: &[Change], record: Option<&ReleaseRecord>) {
        let mut levels = BTreeMap::<usize, Level>::new();

        for (node, level) in changes.iter().flat_map(|change| &change.levels) {
            let entry = levels.entry(*node).or_insert(Level::None);

            *entry = (*entry).max(*level);
        }

        for (index, node) in self.repository.projects() {
            if node.language.version_source() != VersionSource::FileNames {
                continue;
            }

            let Some(current) = &node.package.version else {
                continue;
            };
            let previous = record
                .and_then(|record| record.versions.get(&node.id))
                .unwrap_or(current);
            let Some(movement) = Level::between(previous, current) else {
                continue;
            };
            let id = &node.id;
            let error = match levels.get(&index).copied() {
                Some(level) if level > Level::Patch && level != movement => {
                    format!(
                        "{id} is named `{level}`, but its version moved {previous} -> {current} ({movement})"
                    )
                }
                Some(level) if level <= Level::Patch && movement > Level::Patch => {
                    format!(
                        "{id} moved {previous} -> {current}, a {movement} change, but is named `{level}`"
                    )
                }
                None if movement > Level::Patch => {
                    format!("{id} moved {previous} -> {current} but no change file names it")
                }
                _ => continue,
            };

            self.report.errors.push(error);
        }
    }

    /// Requires a change file for every project with release-relevant changes.
    fn unnamed_projects(&mut self, diff: &Diff, changes: &[Change]) {
        let projects_in_changes = changes
            .iter()
            .flat_map(|change| change.levels.iter().map(|(node, _)| *node))
            .collect::<BTreeSet<_>>();

        for (node, files) in &diff.release_files {
            let id = &self.repository.nodes[*node].id;

            if !self.repository.nodes[*node].is_project() || projects_in_changes.contains(node) {
                continue;
            }

            let sample = files
                .iter()
                .take(SAMPLE_FILES)
                .map(|file| file.display().to_string())
                .collect::<Vec<_>>();
            let more = match files.len().checked_sub(SAMPLE_FILES) {
                Some(count) if count > 0 => format!(" and {count} more"),
                _ => String::new(),
            };

            self.report.errors.push(format!(
                "{id} changed since {} ({}{more}) but no change file names it (`reman change {id}:<level>`, `none` if it needs no release)",
                Git::short_id(&diff.base),
                sample.join(", "),
            ));
        }
    }

    fn orphans(&mut self, diff: &Diff) {
        if diff.orphans.is_empty() {
            return;
        }

        let sample = diff
            .orphans
            .iter()
            .take(SAMPLE_ORPHANS)
            .map(|file| file.display().to_string())
            .collect::<Vec<_>>();

        self.report.notes.push(format!(
            "{} changed file(s) belong to no package: {}",
            diff.orphans.len(),
            sample.join(", ")
        ));
    }
}
