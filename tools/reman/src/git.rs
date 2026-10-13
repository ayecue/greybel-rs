//! Git history reman reads, through gitoxide: commits, the files changed since one, a file at a commit, tags
//! and the `origin` remote. reman never writes to the repository; tags are created on GitHub (release/publish.rs).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gix::bstr::ByteSlice;
use gix::hash::ObjectId;
use gix::revision::walk::Sorting;
use gix::status::UntrackedFiles;
use gix::status::tree_index::TrackRenames;
use gix::traverse::commit::simple::CommitTimeOrder;

/// Git repository, its paths relative to a folder of its working tree (reman's root).
pub struct Git {
    repository: gix::Repository,
    /// Folder, relative to the working tree's root.
    prefix: PathBuf,
}

impl Git {
    /// Number of commit ID characters shown in output.
    const SHORT_ID: usize = 10;

    /// Formats a shortened `id` for display.
    pub fn short_id(id: &str) -> &str {
        &id[..id.len().min(Self::SHORT_ID)]
    }

    /// Repository containing `folder`, with paths relative to `folder`.
    pub fn open(folder: &Path) -> Result<Self> {
        let repository = gix::discover(folder)
            .with_context(|| format!("{} is in no git repository", folder.display()))?;
        let workdir = repository
            .workdir()
            .context("the repository has no working tree")?;
        let folder = std::fs::canonicalize(folder)?;
        let prefix = folder
            .strip_prefix(std::fs::canonicalize(workdir)?)
            .unwrap_or(Path::new(""))
            .to_path_buf();

        Ok(Git { repository, prefix })
    }

    /// Root of its working tree.
    pub fn workdir(&self) -> Result<PathBuf> {
        let workdir = self
            .repository
            .workdir()
            .context("the repository has no working tree")?;

        Ok(workdir.to_path_buf())
    }

    /// Commit `revision` names, as a hex id.
    pub fn resolve(&self, revision: &str) -> Result<String> {
        Ok(self.commit(revision)?.to_string())
    }

    pub fn head(&self) -> Result<String> {
        self.resolve("HEAD")
    }

    /// Newest commit reachable from HEAD that changed `path` (relative to the root): one whose `path` differs from
    /// every parent's, as `git log -1 -- <path>` finds it.
    pub fn last_commit_of(&self, path: &Path) -> Result<Option<String>> {
        let Ok(head) = self.repository.head_id() else {
            return Ok(None);
        };
        let walk = self
            .repository
            .rev_walk([head.detach()])
            .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
            .all()?;

        for info in walk {
            let info = info?;
            let entry = self.entry_at(info.id, &self.prefix.join(path))?;
            let parents = info
                .parent_ids()
                .map(|parent| self.entry_at(parent.detach(), &self.prefix.join(path)))
                .collect::<Result<Vec<_>>>()?;
            let changes_path = match parents.as_slice() {
                [] => entry.is_some(),
                parents => parents.iter().all(|parent| *parent != entry),
            };

            if changes_path {
                return Ok(Some(info.id.to_string()));
            }
        }

        Ok(None)
    }

    /// Lists root-relative working-tree files differing from `base`, including untracked files,
    /// using `git diff --name-only <base>` and `git ls-files --others --exclude-standard`.
    pub fn changed_files(&self, base: &str) -> Result<Vec<PathBuf>> {
        let base_commit = self.commit(base)?;
        let base = self.repository.find_commit(base_commit)?.tree()?;
        let head = self.repository.head_commit()?.tree()?;
        let mut options = gix::diff::Options::default().with_rewrites(None);

        options.track_path();

        let mut files = Vec::new();

        // Changes of trees come with those of the files inside them.
        for change in self.repository.diff_tree_to_tree(&base, &head, options)? {
            if !change.entry_mode().is_tree() {
                files.extend(self.relative(change.location())?);
            }
        }

        // File with uncommitted changes may be back at its content at `base`; one without differs from it.
        for file in self.status()? {
            if self.entry_at(base_commit, &self.prefix.join(&file))? != self.worktree_blob(&file)? {
                files.push(file);
            } else {
                files.retain(|other| *other != file);
            }
        }

        files.sort();
        files.dedup();

        Ok(files)
    }

    /// Text of `path` (relative to the root) at `revision`, if it existed there.
    pub fn show(&self, revision: &str, path: &Path) -> Result<Option<String>> {
        let tree = self
            .repository
            .find_commit(self.commit(revision)?)?
            .tree()?;
        let Some(entry) = tree.lookup_entry_by_path(self.prefix.join(path))? else {
            return Ok(None);
        };

        Ok(Some(
            String::from_utf8_lossy(&entry.object()?.data).into_owned(),
        ))
    }

    pub fn has_tag(&self, tag: &str) -> Result<bool> {
        Ok(self
            .repository
            .try_find_reference(format!("refs/tags/{tag}").as_str())?
            .is_some())
    }

    /// Returns root-relative paths with uncommitted changes, including untracked files.
    pub fn status(&self) -> Result<Vec<PathBuf>> {
        let items = self
            .repository
            .status(gix::progress::Discard)?
            .untracked_files(UntrackedFiles::Files)
            .tree_index_track_renames(TrackRenames::Disabled)
            .into_iter(Vec::new())?;
        let mut paths = Vec::new();

        for item in items {
            paths.extend(self.relative(item?.location())?);
        }

        paths.sort();

        Ok(paths)
    }

    /// Extracts `owner/name` when the `origin` remote points to GitHub.
    pub fn github_repository(&self) -> Option<String> {
        let remote = self.repository.find_remote("origin").ok()?;
        let url = remote.url(gix::remote::Direction::Fetch)?;

        if url.host() != Some("github.com") {
            return None;
        }

        let path = url.path.to_str().ok()?;

        Some(
            path.trim_start_matches('/')
                .trim_end_matches(".git")
                .to_owned(),
        )
    }

    /// Path git reports (relative to the working tree's root) relative to the folder, if it is inside it.
    fn relative(&self, location: &gix::bstr::BStr) -> Result<Option<PathBuf>> {
        let path = location.to_path()?;

        Ok(path.strip_prefix(&self.prefix).ok().map(Path::to_path_buf))
    }

    fn commit(&self, revision: &str) -> Result<ObjectId> {
        let object = self
            .repository
            .rev_parse_single(revision)
            .with_context(|| format!("`{revision}` names no commit"))?
            .object()?;

        Ok(object.peel_to_commit()?.id)
    }

    /// Blob id of the working tree's file at `path` (relative to the root), if it is a file.
    fn worktree_blob(&self, path: &Path) -> Result<Option<ObjectId>> {
        let file = self.workdir()?.join(&self.prefix).join(path);

        if !file.is_file() {
            return Ok(None);
        }

        let data = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;

        Ok(Some(gix::objs::compute_hash(
            self.repository.object_hash(),
            gix::objs::Kind::Blob,
            &data,
        )?))
    }

    /// Object at `path` in the tree of `commit`.
    fn entry_at(&self, commit: ObjectId, path: &Path) -> Result<Option<ObjectId>> {
        let tree = self.repository.find_commit(commit)?.tree()?;

        Ok(tree
            .lookup_entry_by_path(path)?
            .map(|entry| entry.object_id()))
    }
}
