use greybel_core::{ColumnSettings, Range};

use super::Context;
use crate::model::Documentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    pub range: Range,
    pub text: String,
    pub documentation: Option<Documentation>,
}

impl Context<'_> {
    pub(crate) fn hover(&self) -> Option<Hover> {
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
            documentation: self.analysis.documentation(ty),
        })
    }
}
