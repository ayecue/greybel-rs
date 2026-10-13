//! Publishes missing version tags and GitHub releases at HEAD for released apps through the GitHub API.

use anyhow::{Context, Result, bail};
use serde::Serialize;

use super::changelog::Changelog;
use super::github::{GitHub, Release};
use crate::changes::ReleaseRecord;
use crate::git::Git;
use crate::paths::PathExt;
use crate::repository::Repository;

#[derive(Debug, Serialize)]
pub struct Publication {
    pub id: String,
    pub name: String,
    pub version: String,
    pub tag: String,
    pub path: String,
    pub language: String,
}

pub struct PublishOptions {
    /// Print planned releases without creating them.
    pub dry_run: bool,
    pub draft: bool,
    /// Read tags only from the local repository, not GitHub.
    pub offline: bool,
}

/// Publishes all released repository apps at HEAD.
pub struct Publisher<'a> {
    repository: &'a Repository,
    options: &'a PublishOptions,
    git: Git,
    /// Skips remote lookups when `--offline` is set.
    github: Option<GitHub>,
    record: ReleaseRecord,
}

impl<'a> Publisher<'a> {
    pub fn new(repository: &'a Repository, options: &'a PublishOptions) -> Result<Self> {
        let folder = repository.changes();
        let Some(record) = folder.record()? else {
            bail!(
                "no {} yet: prepare a release first",
                folder.record_path().display()
            );
        };
        let github = if options.offline {
            None
        } else {
            Some(GitHub::connect(&repository.settings.github)?)
        };

        Ok(Publisher {
            repository,
            options,
            git: Git::open(&repository.root)?,
            github,
            record,
        })
    }

    /// Creates missing version tags and releases for released apps, writing each result to `log`.
    pub fn publish(&self, log: &mut dyn std::io::Write) -> Result<Vec<Publication>> {
        if !self.git.status()?.is_empty() {
            if !self.options.dry_run {
                bail!(
                    "the working tree has uncommitted changes: publish tags HEAD, so commit the release first"
                );
            }

            writeln!(
                log,
                "note: the working tree has uncommitted changes; a real run refuses to tag"
            )?;
        }

        if !self.options.dry_run && !GitHub::has_token() {
            bail!("publishing needs a GitHub token in GITHUB_TOKEN or GH_TOKEN");
        }

        let commit = self.git.head()?;
        let mut publications = Vec::new();

        for (index, node) in self.repository.projects() {
            if node.is_released()
                && let Some(publication) = self.publish_app(index, &commit, log)?
            {
                publications.push(publication);
            }
        }

        Ok(publications)
    }

    /// Releases app `index` at `commit` if its version is untagged.
    fn publish_app(
        &self,
        index: usize,
        commit: &str,
        log: &mut dyn std::io::Write,
    ) -> Result<Option<Publication>> {
        let node = &self.repository.nodes[index];
        let version = node
            .package
            .version
            .clone()
            .with_context(|| format!("{} has no version", node.id))?;
        let record_version = self.record.versions.get(&node.id);

        if record_version != Some(&version) {
            bail!(
                "{} is at {version} but release.toml says {record_version:?}: prepare and commit the release first",
                node.id
            );
        }

        let tag = self.repository.tag(index, &version);

        if self.has_tag(&tag)? {
            return Ok(None);
        }

        let changelog = std::fs::read_to_string(self.repository.root.join(node.changelog()))
            .unwrap_or_default();
        let notes = Changelog::new(&changelog)
            .notes_of(&version)
            .unwrap_or_default();
        let title = format!("{} {version}", node.name);
        let release = Release {
            tag: &tag,
            commit,
            title: &title,
            notes: &notes,
            draft: self.options.draft,
            latest: node.project.as_ref().is_some_and(|project| project.latest),
        };

        writeln!(
            log,
            "{}tag {tag} at {} and release \"{title}\" on {}{}{}",
            if self.options.dry_run {
                "would create: "
            } else {
                "creating: "
            },
            Git::short_id(commit),
            self.repository.settings.github,
            if release.draft { " (draft)" } else { "" },
            if release.latest { " (latest)" } else { "" },
        )?;

        if !self.options.dry_run {
            self.github
                .as_ref()
                .context("publishing needs GitHub (drop --offline)")?
                .create_release(&release)?;
        }

        Ok(Some(Publication {
            id: node.id.clone(),
            name: node.name.clone(),
            version,
            tag,
            path: node.path().to_slash(),
            language: node.language.name().to_owned(),
        }))
    }

    /// Checks `tag` locally and, unless offline, on GitHub.
    fn has_tag(&self, tag: &str) -> Result<bool> {
        if self.git.has_tag(tag)? {
            return Ok(true);
        }

        self.github
            .as_ref()
            .map_or(Ok(false), |github| github.has_tag(tag))
    }
}
