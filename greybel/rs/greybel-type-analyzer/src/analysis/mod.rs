mod references;

use greybel_core::{Dialect, NodeId};
use greybel_documents::SourceFile;
pub use references::ReferenceTable;

use crate::api::Api;
use crate::diagnostic::Diagnostic;
use crate::model::{DocumentIndex, Documentation, ScopeId, SemanticModel};
use crate::types::{Type, TypeDisplay, TypeId, TypeNotation, TypeTable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    api: Api,
    types: TypeTable,
    model: SemanticModel,
    node_types: Vec<Vec<Option<TypeId>>>,
    references: ReferenceTable,
    diagnostics: Vec<Vec<Diagnostic>>,
    documents: Vec<SourceFile>,
}

impl Analysis {
    pub(crate) fn new(
        api: Api,
        types: TypeTable,
        model: SemanticModel,
        node_types: Vec<Vec<Option<TypeId>>>,
        references: ReferenceTable,
        diagnostics: Vec<Vec<Diagnostic>>,
    ) -> Self {
        Self {
            api,
            types,
            model,
            node_types,
            references,
            diagnostics,
            documents: Vec::new(),
        }
    }

    pub(crate) fn set_documents(&mut self, documents: Vec<SourceFile>) {
        self.documents = documents;
    }

    pub(crate) fn add_diagnostics(
        &mut self,
        document: DocumentIndex,
        diagnostics: impl IntoIterator<Item = Diagnostic>,
    ) {
        let index = document as usize;

        if self.diagnostics.len() <= index {
            self.diagnostics.resize_with(index + 1, Vec::new);
        }

        let list = &mut self.diagnostics[index];

        list.extend(diagnostics);
        Diagnostic::sort(list);
    }

    #[must_use]
    pub fn api(&self) -> Api {
        self.api
    }

    #[must_use]
    pub fn dialect(&self) -> Dialect {
        self.api.dialect()
    }

    #[must_use]
    pub fn types(&self) -> &TypeTable {
        &self.types
    }

    #[must_use]
    pub fn model(&self) -> &SemanticModel {
        &self.model
    }

    #[must_use]
    pub fn references(&self) -> &ReferenceTable {
        &self.references
    }

    #[must_use]
    pub fn diagnostics(&self, document: DocumentIndex) -> &[Diagnostic] {
        self.diagnostics
            .get(document as usize)
            .map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn documents(&self) -> &[SourceFile] {
        &self.documents
    }

    #[must_use]
    pub fn document(&self, file: SourceFile) -> Option<DocumentIndex> {
        let index = self
            .documents
            .iter()
            .position(|&stored_file| stored_file == file)?;

        DocumentIndex::try_from(index).ok()
    }

    #[must_use]
    pub fn node_type(&self, document: DocumentIndex, node: NodeId) -> Option<TypeId> {
        self.node_types
            .get(document as usize)?
            .get(node.index())
            .copied()
            .flatten()
    }

    #[must_use]
    pub fn variable_type(&self, scope: ScopeId, name: &str) -> Option<TypeId> {
        let slot = self.model.variable(scope, name)?;

        Some(self.model.slot_type(slot))
    }

    #[must_use]
    pub fn global_type(&self, name: &str) -> Option<TypeId> {
        self.variable_type(ScopeId::Global, name)
    }

    #[must_use]
    pub fn display(&self, ty: TypeId) -> TypeDisplay<'_> {
        self.notation_in(&self.types).display(ty)
    }

    #[must_use]
    pub fn documentation(&self, ty: TypeId) -> Option<Documentation> {
        self.documentation_in(&self.types, ty)
    }

    pub(crate) fn notation_in<'a>(&'a self, types: &'a TypeTable) -> TypeNotation<'a> {
        TypeNotation::new(&self.model, types)
    }

    pub(crate) fn documentation_in(&self, types: &TypeTable, ty: TypeId) -> Option<Documentation> {
        match types.get(ty) {
            Type::Function(function) => self.model.documentation(self.api, *function),
            _ => None,
        }
    }
}
