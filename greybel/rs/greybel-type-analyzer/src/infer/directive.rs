use greybel_core::{NodeId, NodeKind};

use super::Inference;
use crate::types::TypeId;

impl Inference<'_> {
    pub(crate) fn infer_directive(&mut self, node: NodeId) -> TypeId {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Import(import) => {
                let exports = self.infer_dependency(node).unwrap_or(TypeId::ANY);

                if let Some(name) = self.identifier_name(import.name) {
                    let slot = self.model.insert_variable(self.current_scope, name);

                    self.widen(slot, exports);
                    self.add_definition(import.name, &[slot]);

                    let ty = self.model.slot_type(slot);

                    self.record_node_type(import.name, ty);
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
