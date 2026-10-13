use std::sync::Arc;

use greybel_core::{Ast, NodeId};
use vscode_uri::Uri;

use crate::import::ImportKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub(crate) u32);

impl ModuleId {
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub node: NodeId,
    pub kind: ImportKind,
    pub module: ModuleId,
}

pub struct Module {
    pub uri: Uri,
    pub ast: Arc<Ast>,
    imported: bool,
    dependencies: Vec<Edge>,
    injections: Vec<(NodeId, Arc<str>)>,
}

impl Module {
    pub(crate) fn new(uri: Uri, ast: Arc<Ast>) -> Self {
        Self {
            uri,
            ast,
            imported: false,
            dependencies: Vec::new(),
            injections: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_imported(&self) -> bool {
        self.imported
    }

    #[must_use]
    pub fn dependency(&self, node: NodeId) -> Option<ModuleId> {
        self.dependencies
            .iter()
            .find(|edge| edge.node == node)
            .map(|edge| edge.module)
    }

    #[must_use]
    pub fn dependencies(&self) -> &[Edge] {
        &self.dependencies
    }

    #[must_use]
    pub fn injection(&self, node: NodeId) -> Option<&str> {
        self.injections
            .iter()
            .find(|(inject, _)| *inject == node)
            .map(|(_, content)| &**content)
    }

    pub(crate) fn register_import(&mut self) {
        self.imported = true;
    }

    pub(crate) fn add_dependency(&mut self, edge: Edge) {
        self.dependencies.push(edge);
    }

    pub(crate) fn add_injection(&mut self, node: NodeId, content: Arc<str>) {
        self.injections.push((node, content));
    }
}
