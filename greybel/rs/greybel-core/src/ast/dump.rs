use std::borrow::Cow;

use greybel_symbols::literals::NULL;

use super::{AstNode, Atom, AtomTable, NodeId, NodeList, NodeListStorage, NodeStore, TextSpan};
use crate::position::{ColumnSettings, LineIndex, Span};

pub struct DumpContext<'a> {
    location: String,
    source: &'a str,
    atoms: &'a AtomTable,
    lists: &'a NodeListStorage,
    child: &'a dyn Fn(NodeId) -> String,
}

impl DumpContext<'_> {
    #[must_use]
    pub fn resolve(&self, atom: Atom) -> &str {
        self.atoms.resolve(atom)
    }

    #[must_use]
    pub fn read(&self, text: TextSpan) -> Cow<'_, str> {
        text.read(self.source)
    }

    #[must_use]
    pub fn location(&self) -> &str {
        &self.location
    }

    #[must_use]
    pub fn child(&self, id: NodeId) -> String {
        (self.child)(id)
    }

    #[must_use]
    pub fn optional(&self, id: Option<NodeId>) -> String {
        id.map_or_else(|| NULL.to_owned(), |id| self.child(id))
    }

    #[must_use]
    pub fn ids(&self, list: NodeList) -> &[NodeId] {
        self.lists.get(list)
    }

    #[must_use]
    pub fn list(&self, list: NodeList) -> String {
        self.ids(list)
            .iter()
            .map(|&id| self.child(id))
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[must_use]
    pub fn block(&self, list: NodeList) -> String {
        self.ids(list)
            .iter()
            .map(|&id| self.child(id))
            .collect::<Vec<_>>()
            .join("\n")
            .split('\n')
            .map(|line| format!("\t{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(super) fn render(
    nodes: &NodeStore,
    source: &str,
    lines: &LineIndex,
    columns: ColumnSettings,
    id: NodeId,
) -> String {
    let node = nodes.get(id);
    let child = |id: NodeId| render(nodes, source, lines, columns, id);
    let ctx = DumpContext {
        location: span_location(lines, columns, node.span),
        source,
        atoms: nodes.atoms(),
        lists: nodes.lists(),
        child: &child,
    };

    node.kind.dump(&ctx)
}

#[must_use]
pub fn span_location(lines: &LineIndex, columns: ColumnSettings, span: Span) -> String {
    let range = lines.range(span, columns);

    format!(
        "{}:{}-{}:{}",
        range.start.line, range.start.character, range.end.line, range.end.character
    )
}
