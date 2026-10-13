//! Repository as reman sees it: the `reman.toml` files, the workspaces they declare, every package of those
//! workspaces (projects or not), and what depends on what.

mod graph;
mod node;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
pub use graph::{Edge, Input, Via};
use indexmap::IndexMap;
pub use node::{Node, Workspace};

use crate::changes::ChangeFolder;
use crate::config::{self, File, LanguageTable, ProjectBase, RepositoryTable, Select, TaskTable};
use crate::language::Language;
use crate::paths::PathExt;

pub struct Repository {
    pub root: PathBuf,
    pub settings: RepositoryTable,
    /// Root file's repository tasks.
    pub tasks: IndexMap<String, TaskTable>,
    /// Root file's language defaults.
    pub languages: IndexMap<&'static dyn Language, LanguageTable>,
    pub workspaces: Vec<Workspace>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// External folders read by packages.
    pub inputs: Vec<Input>,
    pub problems: Vec<Problem>,
    /// Root-relative `reman.toml` paths.
    pub files: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub error: bool,
    pub message: String,
}

impl Problem {
    pub fn error(message: String) -> Self {
        Problem {
            error: true,
            message,
        }
    }
}

impl Repository {
    /// Repository whose root (the folder of the `reman.toml` with `[repository]`) contains `start`.
    pub fn find(start: &Path) -> Result<Self> {
        let mut folder = (std::env::current_dir()?.join(start)).normalize();

        loop {
            let candidate = folder.join(config::FILE_NAME);

            if candidate.is_file() && File::read(&candidate)?.repository.is_some() {
                return Self::load(&folder);
            }

            if !folder.pop() {
                bail!(
                    "no reman.toml with [repository] in {} or above",
                    start.display()
                );
            }
        }
    }

    pub fn load(root: &Path) -> Result<Self> {
        Self::load_with(root, &[])
    }

    /// Repository as it would be with `overlay`'s files (relative to the root) written.
    pub fn load_with(root: &Path, overlay: &[(PathBuf, String)]) -> Result<Self> {
        let root = root.normalize();
        let mut files = File::read_all(&root, overlay)?;
        let Some((_, root_file)) = files.remove(Path::new("")) else {
            bail!("{} has no reman.toml", root.display());
        };
        let Some(settings) = root_file.repository else {
            bail!("the root reman.toml has no [repository]");
        };

        if root_file.workspace.is_some() || root_file.project.is_some() {
            bail!("the root reman.toml declares the repository only");
        }

        let root_defaults =
            ProjectBase::root().over(settings.project.as_ref(), Path::new(config::FILE_NAME));
        let mut workspaces = Vec::new();

        for (folder, (path, file)) in &files {
            if file.repository.is_some() {
                bail!("{}: only the root declares [repository]", path.display());
            }

            if !file.languages.is_empty() {
                bail!("{}: only the root declares [languages]", path.display());
            }

            if let Some(workspace) = &file.workspace {
                workspaces.push(Workspace {
                    path: folder.clone(),
                    language: workspace.language,
                    development: workspace.development,
                    defaults: root_defaults.over(workspace.project.as_ref(), path),
                    tasks: if file.project.is_none() {
                        file.tasks.clone()
                    } else {
                        IndexMap::new()
                    },
                    origin: path.clone(),
                });
            }
        }

        let mut nodes: Vec<Node> = Vec::new();
        let mut by_path = BTreeMap::<PathBuf, usize>::new();

        for (index, workspace) in workspaces.iter().enumerate() {
            let packages = workspace
                .language
                .packages(&root, &workspace.path)
                .with_context(|| {
                    format!(
                        "reading the {} workspace {}",
                        workspace.language,
                        workspace.path.display()
                    )
                })?;

            for package in packages {
                if let Some(other) = by_path.get(&package.path) {
                    bail!(
                        "{} is a package of both {} and {}",
                        package.path.display(),
                        workspaces[nodes[*other].workspace].path.display(),
                        workspace.path.display()
                    );
                }

                let (project, tasks) = match files.get(&package.path) {
                    Some((path, file)) if file.project.is_some() => (
                        Some(
                            workspace
                                .defaults
                                .project(file.project.as_ref().unwrap(), path)?,
                        ),
                        file.tasks.clone(),
                    ),
                    _ => (None, IndexMap::new()),
                };

                by_path.insert(package.path.clone(), nodes.len());
                nodes.push(Node {
                    id: format!("{}/{}", workspace.language.prefix(), package.name),
                    name: package.name.clone(),
                    language: workspace.language,
                    workspace: index,
                    defaults: workspace.defaults.clone(),
                    package,
                    project,
                    tasks,
                });
            }
        }

        let mut ids = BTreeSet::new();

        if let Some(node) = nodes.iter().find(|node| !ids.insert(node.id.as_str())) {
            bail!("two packages are {}", node.id);
        }

        let problems = files
            .iter()
            .filter(|(folder, (_, file))| file.project.is_some() && !by_path.contains_key(*folder))
            .map(|(folder, (path, _))| {
                Problem::error(match Workspace::closest(&workspaces, folder) {
                    None => format!("{}: a [project] outside every [workspace]", path.display()),
                    Some(workspace) => format!(
                        "{}: no package of the {} workspace {} lives here",
                        path.display(),
                        workspaces[workspace].language,
                        workspaces[workspace].path.display()
                    ),
                })
            })
            .collect();
        let mut repository = Repository {
            root,
            settings,
            tasks: root_file.tasks,
            languages: root_file.languages,
            workspaces,
            nodes,
            edges: Vec::new(),
            inputs: Vec::new(),
            problems,
            files: files.into_values().map(|(path, _)| path).collect(),
        };

        repository.link(&by_path);

        Ok(repository)
    }

    /// Package an id (`rs/greybel-core`) or unique name (`greybel-core`) names.
    pub fn resolve(&self, name: &str) -> Result<usize> {
        if let Some(index) = self.nodes.iter().position(|node| node.id == name) {
            return Ok(index);
        }

        let matches = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.name == name)
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [(index, _)] => Ok(*index),
            [] => bail!("no package is `{name}`"),
            _ => bail!(
                "`{name}` is ambiguous: {}",
                matches
                    .iter()
                    .map(|(_, node)| node.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// Changelog package label: name, or ID when names collide.
    pub fn display_name(&self, node: usize) -> &str {
        let name = &self.nodes[node].name;

        if self
            .nodes
            .iter()
            .filter(|other| &other.name == name)
            .count()
            > 1
        {
            &self.nodes[node].id
        } else {
            name
        }
    }

    pub fn projects(&self) -> impl Iterator<Item = (usize, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.is_project())
    }

    /// Changes folder: change files and the release record.
    pub fn changes(&self) -> ChangeFolder<'_> {
        ChangeFolder::new(self, PathBuf::from(&self.settings.changes))
    }

    pub fn workspace_paths(&self, language: &'static dyn Language) -> Vec<PathBuf> {
        self.workspaces
            .iter()
            .filter(|workspace| workspace.language == language)
            .map(|workspace| workspace.path.clone())
            .collect()
    }

    pub fn select(&self, workspace: usize) -> Option<&Select> {
        self.languages
            .get(&self.workspaces[workspace].language)
            .and_then(|language| language.select.as_ref())
    }

    /// Tag of a project's release at `version`: its own `tag`, else the repository's.
    pub fn tag(&self, node: usize, version: &str) -> String {
        let node = &self.nodes[node];
        let template = node
            .project
            .as_ref()
            .and_then(|project| project.tag.as_deref())
            .unwrap_or(&self.settings.tag);

        template
            .replace("{name}", &node.name)
            .replace("{version}", version)
    }
}
