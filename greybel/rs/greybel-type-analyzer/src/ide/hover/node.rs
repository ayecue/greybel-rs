use std::fmt::Write as _;

use greybel_core::{ColumnSettings, NodeId};

use super::{Hover, HoverKind};
use crate::ide::Context;
use crate::model::Site;
use crate::types::TypeId;

impl Context<'_> {
    pub(super) fn hover(&self) -> Option<Hover> {
        let (node, ty) = self
            .ast
            .path_at(self.offset)
            .into_iter()
            .rev()
            .find_map(|node| Some((node, self.analysis.node_type(self.document, node)?)))?;
        let notation = self.analysis.notation_in(self.analysis.types());
        let mut text = match self.ast.name(node) {
            Some(name) => format!("{name}: {}", notation.display(ty)),
            None => notation.display(ty).to_string(),
        };

        if let Some(declared_type) = self.declared_type_at_definition(node)
            && declared_type != ty
        {
            let _ = write!(text, " (declared: {})", notation.display(declared_type));
        }

        if let Some(content) = notation.map_content(ty) {
            text.push(' ');
            text.push_str(&content);
        }

        Some(Hover {
            range: self
                .ast
                .lines()
                .range(self.ast.span(node), ColumnSettings::default()),
            text,
            kind: HoverKind::Signature,
            documentation: self.analysis.documentation(ty),
        })
    }

    fn declared_type_at_definition(&self, node: NodeId) -> Option<TypeId> {
        let site = Site {
            document: self.document,
            node,
        };
        let references = self.analysis.references();
        let &[slot] = references.slots(site) else {
            return None;
        };

        references
            .definition_sites(slot)
            .contains(&site)
            .then(|| self.analysis.model().slot_type(slot))
    }
}
