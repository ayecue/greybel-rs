//! [`Preparation`] applies planned version and changelog edits, writes the release record, and consumes change files;
//! [`Publisher`] creates missing version tags and GitHub releases for released apps.

pub mod changelog;
pub mod plan;

mod github;
mod publish;

use anyhow::{Context, Result};
use changelog::Changelog;
use jiff::civil::Date;
use plan::{Plan, Release};
pub use publish::{Publication, PublishOptions, Publisher};

use crate::changes::ReleaseRecord;
use crate::edits::EditTable;
use crate::language::{VersionChange, VersionSource};
use crate::repository::Repository;

/// Plan of the pending change files and the edits carrying it out.
pub struct Preparation {
    pub plan: Plan,
    pub edits: EditTable,
}

impl Preparation {
    /// Release of the pending change files on `date`.
    pub fn new(repository: &Repository, date: Date) -> Result<Self> {
        let mut preparation = Preparation {
            plan: Plan::from_change_files(repository)?,
            edits: EditTable::new(&repository.root),
        };

        if preparation.plan.is_empty() {
            return Ok(preparation);
        }

        for release in &preparation.plan.releases {
            Self::write_release(repository, release, date, &mut preparation.edits)?;
        }

        let release_folder = repository.changes().release_path(date);

        for change in &preparation.plan.change_files {
            let text = preparation
                .edits
                .read(change)?
                .with_context(|| format!("{} is missing", change.display()))?;
            let name = change.file_name().context("a change file has a name")?;

            preparation.edits.write(&release_folder.join(name), text);
            preparation.edits.delete(change);
        }

        let record = preparation.record(repository, date);

        preparation
            .edits
            .write(&repository.changes().record_path(), record.render()?);

        Ok(preparation)
    }

    /// Updates a release version at its reman-managed location and writes its changelog section.
    fn write_release(
        repository: &Repository,
        release: &Release,
        date: Date,
        edits: &mut EditTable,
    ) -> Result<()> {
        let node = &repository.nodes[release.node];

        if node.language.version_source() == VersionSource::Manifest
            && release.version != release.previous
        {
            let change = VersionChange {
                package: &node.package,
                previous: &release.previous,
                version: &release.version,
                workspaces: &repository.workspace_paths(node.language),
            };

            node.language
                .write_version(&change, edits)
                .with_context(|| format!("writing the version of {}", node.id))?;
        }

        let path = node.changelog();
        let section = release.section(repository, date);
        let current = edits
            .read(&path)?
            .unwrap_or_else(|| Changelog::empty(&node.name));
        let text = Changelog::new(&current).with_section(&section);

        edits.write(&path, text);

        Ok(())
    }

    /// Post-release versions by project.
    fn record(&self, repository: &Repository, date: Date) -> ReleaseRecord {
        let mut record = ReleaseRecord {
            date,
            ..ReleaseRecord::default()
        };

        for (index, node) in repository.projects() {
            let version = self
                .plan
                .version_of(index)
                .map(str::to_owned)
                .or_else(|| node.package.version.clone());

            if let Some(version) = version {
                record.versions.insert(node.id.clone(), version);
            }
        }

        record
    }
}
