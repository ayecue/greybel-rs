use greybel_core::{NodeId, NodeKind};
use greybel_documents::ImportKind;
use greybel_symbols::names::{EXPORTS, MODULE};

use super::Inference;
use crate::diagnostic::DiagnosticKind;
use crate::model::{DocumentIndex, KeyRef, MapOrigin};
use crate::types::TypeId;

impl Inference<'_> {
    pub(crate) fn dependency(&self, node: NodeId) -> Option<(ImportKind, DocumentIndex)> {
        self.documents[self.current_document as usize]
            .dependencies
            .get(&node)
            .copied()
    }

    pub(crate) fn infer_dependency(&mut self, node: NodeId) -> Option<TypeId> {
        let (kind, target) = self.dependency(node)?;

        if !kind.runs_code() {
            return None;
        }

        if kind == ImportKind::NativeImport && self.builds_own_file && !self.is_at_head(node) {
            self.report(
                DiagnosticKind::MisplacedImportCode,
                node,
                "Place this \"import_code\" at the start of the file; the build moves it to the entry file before this file's code.".to_owned(),
            );
        }

        let top_level = self.top_level_scopes[target as usize];
        let module = (kind == ImportKind::Import).then(|| {
            let map = self.model.insert_map(MapOrigin::Module(target));

            self.object_type(map)
        });

        if self.pass.visited_documents.insert(target) {
            let previous = (self.current_document, self.current_scope);
            // A module runs in its own scope; the importing code sees only its side effect.
            let importer = module.map(|_| self.enter_body());

            self.current_document = target;
            if kind == ImportKind::Import {
                self.current_scope = top_level;
            }

            if let Some(module) = module {
                let slot = self.model.insert_variable(top_level, MODULE);

                self.widen(slot, module);
                self.unassign_variables(top_level, &[slot]);
            }

            let body = self.ast().body();

            let previous_own_file = self.builds_own_file;

            self.builds_own_file |= kind == ImportKind::NativeImport;
            self.begin_side_effect(module.map(|_| top_level));
            self.infer_block(body);
            self.builds_own_file = previous_own_file;
            (self.current_document, self.current_scope) = previous;

            let side_effect = self.end_document_side_effect(target);

            if let Some(importer) = importer {
                self.exit_body(importer);
                self.apply_side_effect(&side_effect);
            }
        } else if matches!(kind, ImportKind::Include | ImportKind::Import)
            && let Some(side_effect) = self.document_side_effect(target)
        {
            // An include runs its code again; a module runs once, and which import runs it first is not known.
            self.apply_side_effect(&side_effect);
        }

        let module = module?;

        Some(self.lookup().member_type(module, KeyRef::String(EXPORTS)))
    }

    fn is_at_head(&self, node: NodeId) -> bool {
        let ast = self.ast();
        let body = ast.body();
        let Some(position) = body.iter().position(|&statement| statement == node) else {
            return false;
        };

        body[..position].iter().all(|&statement| {
            matches!(
                ast.kind(statement),
                NodeKind::ImportCode(_) | NodeKind::Noop(_)
            )
        })
    }

    pub(crate) fn infer_directive(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Import(import) => {
                let exports = self.infer_dependency(node).unwrap_or(TypeId::ANY);

                if let Some(name) = self.identifier_name(import.name) {
                    let slot = self.model.insert_variable(self.current_scope, name);

                    self.widen(slot, exports);
                    self.add_assignment(self.current_scope, slot);
                    self.flow.assign(slot, exports);
                    self.add_definition(import.name, &[slot]);

                    self.register_node_type(import.name, exports);
                }

                for &binding in ast.list(import.bindings) {
                    self.infer_statement(binding);
                }

                TypeId::NULL
            }
            NodeKind::Include(_) | NodeKind::ImportCode(_) => {
                self.infer_dependency(node);

                TypeId::NULL
            }
            NodeKind::Envar(_) | NodeKind::Inject(_) | NodeKind::File(_) => TypeId::STRING,
            NodeKind::Line(_) => TypeId::NUMBER,
            _ => TypeId::NULL,
        }
    }
}
