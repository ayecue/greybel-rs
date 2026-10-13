//! Dependency graph: manifest, folder-table path, and `uses` edges, plus
//! orders and closures over them.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use super::{Problem, Repository};
use crate::language::{FileTree, Scope, Target};

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub scope: Scope,
    pub via: Via,
}

#[derive(Clone, Debug)]
pub enum Via {
    Manifest,
    /// Folder-table reference declared in this file.
    Reference(PathBuf),
    /// Project's `uses`.
    Uses,
}

/// Folder outside every package that a package reads (shaders, fixtures): its changes are the package's.
#[derive(Clone, Debug)]
pub struct Input {
    pub node: usize,
    pub path: PathBuf,
    pub scope: Scope,
}

impl Repository {
    /// Converts package dependencies, references, and `uses` declarations into graph edges and inputs.
    pub(super) fn link(&mut self, by_path: &BTreeMap<PathBuf, usize>) {
        let mut edges = Vec::new();
        let mut inputs = Vec::new();
        let mut problems = Vec::new();

        for (from, node) in self.nodes.iter().enumerate() {
            for dependency in &node.package.dependencies {
                let to = match &dependency.target {
                    Target::Path(path) => by_path.get(path).copied().or_else(|| self.owner(path)),
                    Target::Name(name) => self
                        .nodes
                        .iter()
                        .position(|other| other.workspace == node.workspace && &other.name == name),
                };

                match to {
                    Some(to) if to != from => edges.push(Edge {
                        from,
                        to,
                        scope: dependency.scope,
                        via: Via::Manifest,
                    }),
                    Some(_) => {}
                    None => {
                        let target = match &dependency.target {
                            Target::Path(path) => path.display().to_string(),
                            Target::Name(name) => name.clone(),
                        };

                        problems.push(Problem {
                            error: node.is_project() && dependency.scope == Scope::Normal,
                            message: format!(
                                "{} depends on {target}, a package of no declared workspace",
                                node.id
                            ),
                        });
                    }
                }
            }

            // Ignores build output such as `target` when determining shipped package contents.
            for reference in node
                .package
                .references
                .iter()
                .filter(|reference| !FileTree::is_build_output(&reference.path))
            {
                match self.owner(&reference.path) {
                    Some(to) if to != from => edges.push(Edge {
                        from,
                        to,
                        scope: reference.scope,
                        via: Via::Reference(reference.source.clone()),
                    }),
                    Some(_) => {}
                    None if !self
                        .nodes
                        .iter()
                        .any(|other| other.path().starts_with(&reference.path)) =>
                    {
                        inputs.push(Input {
                            node: from,
                            path: reference.path.clone(),
                            scope: reference.scope,
                        });
                    }
                    None => {}
                }
            }

            for name in node.project.iter().flat_map(|project| &project.uses) {
                match self.resolve(name) {
                    Ok(to) => edges.push(Edge {
                        from,
                        to,
                        scope: Scope::Normal,
                        via: Via::Uses,
                    }),
                    Err(error) => {
                        problems.push(Problem::error(format!("{}: `uses`: {error}", node.id)))
                    }
                }
            }
        }

        edges.sort_by_key(|edge| (edge.from, edge.to, edge.scope));
        edges.dedup_by_key(|edge| (edge.from, edge.to, edge.scope));
        self.edges = edges;
        self.inputs = inputs;
        self.problems.extend(problems);
    }

    /// Innermost package whose folder contains `path`.
    pub fn owner(&self, path: &Path) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| path.starts_with(node.path()))
            .max_by_key(|(_, node)| node.path().components().count())
            .map(|(index, _)| index)
    }

    /// Returns packages with direct `scopes` dependencies on `node`.
    pub fn dependents<'a>(
        &'a self,
        node: usize,
        scopes: &'a [Scope],
    ) -> impl Iterator<Item = usize> + 'a {
        self.edges
            .iter()
            .filter(move |edge| edge.to == node && scopes.contains(&edge.scope))
            .map(|edge| edge.from)
    }

    /// Returns `start` plus all direct and transitive dependents through `scopes`.
    pub fn with_dependents(
        &self,
        start: impl IntoIterator<Item = usize>,
        scopes: &[Scope],
    ) -> BTreeSet<usize> {
        let mut visits = BTreeSet::new();
        let mut queue = start.into_iter().collect::<VecDeque<_>>();

        while let Some(node) = queue.pop_front() {
            if visits.insert(node) {
                queue.extend(self.dependents(node, scopes));
            }
        }

        visits
    }

    /// Sorts workspaces after their package dependencies across all scopes, using path order for cycles.
    pub fn workspace_order(&self) -> Vec<usize> {
        let mut order = Order::new(self.workspaces.len());

        for edge in &self.edges {
            order.require(
                self.nodes[edge.from].workspace,
                self.nodes[edge.to].workspace,
            );
        }

        order.sequence(|index| self.workspaces[index].path.clone())
    }

    /// Sorts packages after their dependencies.
    pub fn node_order(&self) -> Vec<usize> {
        let mut order = Order::new(self.nodes.len());

        for edge in &self.edges {
            order.require(edge.from, edge.to);
        }

        order.sequence(|index| self.nodes[index].path().to_path_buf())
    }
}

/// Ordering constraints between indexed items.
struct Order {
    needs: Vec<BTreeSet<usize>>,
}

impl Order {
    fn new(count: usize) -> Self {
        Order {
            needs: vec![BTreeSet::new(); count],
        }
    }

    /// Requires `item` to follow `need`.
    fn require(&mut self, item: usize, need: usize) {
        if item != need {
            self.needs[item].insert(need);
        }
    }

    /// Uses Kahn's algorithm with `key` to break ties and cycles.
    fn sequence(&self, key: impl Fn(usize) -> PathBuf) -> Vec<usize> {
        let count = self.needs.len();
        let mut in_order = vec![false; count];
        let mut order = Vec::with_capacity(count);
        let mut by_key = (0..count).collect::<Vec<_>>();

        by_key.sort_by_key(|index| key(*index));

        while order.len() < count {
            let ready = by_key.iter().copied().find(|index| {
                !in_order[*index] && self.needs[*index].iter().all(|need| in_order[*need])
            });
            let next = ready.unwrap_or_else(|| {
                by_key
                    .iter()
                    .copied()
                    .find(|index| !in_order[*index])
                    .unwrap()
            });

            in_order[next] = true;
            order.push(next);
        }

        order
    }
}
