//! `reman show` and `reman graph` output: project settings with origins, and packages
//! with what they depend on.

use std::fmt;

use crate::language::Scope;
use crate::paths::PathExt;
use crate::repository::{Edge, Repository, Via};

/// Package's settings, with the file each came from, and what it depends on and is used by.
pub struct ProjectView<'a> {
    pub repository: &'a Repository,
    pub node: usize,
}

/// Format `reman graph` prints in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphFormat {
    /// Package per line, its dependencies below it.
    Text,
    /// Graphviz output format.
    Dot,
}

/// Projects (every package and test dependency with `all`) and what they depend on.
pub struct GraphView<'a> {
    pub repository: &'a Repository,
    pub all: bool,
    pub format: GraphFormat,
}

impl fmt::Display for ProjectView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repository = self.repository;
        let node = &repository.nodes[self.node];
        let workspace = &repository.workspaces[node.workspace];

        writeln!(f, "{}  {}", node.id, node.path().to_slash())?;
        writeln!(
            f,
            "  language    {}  ({})",
            node.language,
            workspace.origin.to_slash()
        )?;
        writeln!(
            f,
            "  version     {}",
            node.package.version.as_deref().unwrap_or("(none)")
        )?;

        match &node.project {
            Some(project) => {
                let origin = project.origin.to_slash();

                writeln!(f, "  kind        {}  ({origin})", project.kind)?;
                writeln!(
                    f,
                    "  release     {}  ({origin})",
                    if project.release { "tagged" } else { "no" }
                )?;

                if project.latest {
                    writeln!(f, "  latest      yes  ({origin})")?;
                }
            }
            None => writeln!(f, "  kind        (no [project]: never released)")?,
        }

        let (changelog, skip) = match &node.project {
            Some(project) => (&project.changelog, &project.skip),
            None => (&node.defaults.changelog, &node.defaults.skip),
        };

        writeln!(
            f,
            "  changelog   {}  ({})",
            node.changelog().to_slash(),
            changelog.origin.to_slash()
        )?;
        writeln!(
            f,
            "  skip        {:?}  ({})",
            skip.value,
            skip.origin.to_slash()
        )?;

        for (title, from_node) in [("depends on", true), ("used by", false)] {
            let edges = repository
                .edges
                .iter()
                .filter(|edge| {
                    if from_node {
                        edge.from == self.node
                    } else {
                        edge.to == self.node
                    }
                })
                .collect::<Vec<_>>();

            if !edges.is_empty() {
                writeln!(f, "  {title}:")?;
            }

            for edge in edges {
                let other = if from_node { edge.to } else { edge.from };

                writeln!(f, "    {}{}", repository.nodes[other].id, EdgeLabel(edge))?;
            }
        }

        for input in repository
            .inputs
            .iter()
            .filter(|input| input.node == self.node)
        {
            writeln!(
                f,
                "  reads       {}{}",
                input.path.to_slash(),
                ScopeLabel(input.scope)
            )?;
        }

        for (key, task) in &node.tasks {
            writeln!(
                f,
                "  task        {key} ({})",
                task.suite.as_deref().unwrap_or("no suite")
            )?;
        }

        Ok(())
    }
}

impl GraphView<'_> {
    fn text(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repository = self.repository;

        for index in repository.node_order() {
            let node = &repository.nodes[index];

            if !self.all && !node.is_project() {
                continue;
            }

            let kind = node
                .project
                .as_ref()
                .map_or_else(|| "-".to_owned(), |project| project.kind.to_string());

            writeln!(
                f,
                "{} {} [{kind}]",
                node.id,
                node.package.version.as_deref().unwrap_or("?")
            )?;

            for edge in repository.edges.iter().filter(|edge| edge.from == index) {
                if self.all || edge.scope != Scope::Dev {
                    writeln!(
                        f,
                        "  -> {}{}",
                        repository.nodes[edge.to].id,
                        EdgeLabel(edge)
                    )?;
                }
            }
        }

        Ok(())
    }

    fn dot(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let repository = self.repository;

        writeln!(f, "digraph reman {{\n  rankdir=LR;")?;

        for node in repository
            .nodes
            .iter()
            .filter(|node| self.all || node.is_project())
        {
            writeln!(
                f,
                "  \"{}\" [shape={}];",
                node.id,
                if node.is_project() { "box" } else { "ellipse" }
            )?;
        }

        for edge in &repository.edges {
            let (from, to) = (&repository.nodes[edge.from], &repository.nodes[edge.to]);

            if !self.all && (edge.scope == Scope::Dev || !from.is_project() || !to.is_project()) {
                continue;
            }

            let style = if edge.scope == Scope::Dev {
                " [style=dashed]"
            } else {
                ""
            };

            writeln!(f, "  \"{}\" -> \"{}\"{style};", from.id, to.id)?;
        }

        writeln!(f, "}}")
    }
}

impl fmt::Display for GraphView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.format {
            GraphFormat::Text => self.text(f),
            GraphFormat::Dot => self.dot(f),
        }
    }
}

/// Appends ` (tests)` to edges that reach tests only.
struct ScopeLabel(Scope);

impl fmt::Display for ScopeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Scope::Normal => Ok(()),
            Scope::Dev => f.write_str(" (tests)"),
        }
    }
}

/// Edge's scope and what declares it.
struct EdgeLabel<'a>(&'a Edge);

impl fmt::Display for EdgeLabel<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", ScopeLabel(self.0.scope))?;

        match &self.0.via {
            Via::Manifest => Ok(()),
            Via::Reference(file) => write!(f, " via {}", file.to_slash()),
            Via::Uses => f.write_str(" via uses"),
        }
    }
}
