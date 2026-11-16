use greybel_core::{Ast, NodeId, NodeKind};

use crate::options::NativeImportMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImportKind {
    Import,
    Include,
    TypeInclude,
    NativeImport,
    Inject,
}

impl ImportKind {
    #[must_use]
    pub fn is_dependency(self) -> bool {
        !matches!(self, Self::Inject)
    }

    #[must_use]
    pub fn runs_code(self) -> bool {
        matches!(self, Self::Import | Self::Include | Self::NativeImport)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub node: NodeId,
    pub kind: ImportKind,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportContainer(Vec<Import>);

impl ImportContainer {
    #[must_use]
    pub fn of(ast: &Ast, native_imports: NativeImportMode) -> Self {
        let native = ast
            .native_imports
            .iter()
            .filter_map(|&node| match ast.kind(node) {
                NodeKind::ImportCode(import)
                    if import.eval
                        && native_imports != NativeImportMode::Call
                        && (import.emit || native_imports == NativeImportMode::Evaluation) =>
                {
                    Some(Import {
                        node,
                        kind: ImportKind::NativeImport,
                        target: ast.read(import.directory).into_owned(),
                    })
                }
                _ => None,
            });
        let imports = ast.imports.iter().filter_map(|&node| match ast.kind(node) {
            NodeKind::Import(import) => Some(Import {
                node,
                kind: ImportKind::Import,
                target: ast.read(import.path).into_owned(),
            }),
            _ => None,
        });
        let includes = ast
            .includes
            .iter()
            .filter_map(|&node| match ast.kind(node) {
                NodeKind::Include(include) => Some(Import {
                    node,
                    kind: if include.type_only {
                        ImportKind::TypeInclude
                    } else {
                        ImportKind::Include
                    },
                    target: ast.read(include.path).into_owned(),
                }),
                _ => None,
            });
        let injects = ast.injects.iter().filter_map(|&node| match ast.kind(node) {
            NodeKind::Inject(inject) => Some(Import {
                node,
                kind: ImportKind::Inject,
                target: ast.read(inject.path).into_owned(),
            }),
            _ => None,
        });

        Self(
            native
                .chain(imports)
                .chain(includes)
                .chain(injects)
                .collect(),
        )
    }

    pub fn iter(&self) -> impl Iterator<Item = &Import> {
        self.0.iter()
    }

    pub fn dependencies(&self) -> impl Iterator<Item = &Import> {
        self.iter().filter(|import| import.kind.is_dependency())
    }

    pub fn injections(&self) -> impl Iterator<Item = &Import> {
        self.iter().filter(|import| !import.kind.is_dependency())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a ImportContainer {
    type IntoIter = std::slice::Iter<'a, Import>;
    type Item = &'a Import;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
