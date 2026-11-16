use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;
use crate::model::{Site, SlotId};

impl Inference<'_> {
    pub(crate) fn validate_deprecated_uses(&mut self) {
        let mut uses: Vec<(Site, String)> = Vec::new();

        for (site, slots) in self.references.entries() {
            let Some(text) = slots
                .iter()
                .filter(|&&slot| !self.references.definition_sites(slot).contains(&site))
                .find_map(|&slot| self.deprecation(slot))
            else {
                continue;
            };
            let ast = self.documents[site.document as usize].ast;
            let name = ast.name(site.node).unwrap_or("This function");
            let mut message = format!("\"{name}\" is deprecated.");

            if !text.is_empty() {
                message.push(' ');
                message.push_str(text);
            }

            uses.push((site, message));
        }

        for (site, message) in uses {
            self.report_in(
                site.document,
                DiagnosticKind::Deprecated,
                site.node,
                message,
            );
        }
    }

    fn deprecation(&self, slot: SlotId) -> Option<&str> {
        let ty = self.model.known_slot_type(slot)?;

        self.model.deprecation(&self.types, ty)
    }
}
