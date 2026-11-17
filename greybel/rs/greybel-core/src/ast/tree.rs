use std::borrow::Cow;
use std::fmt;
use std::sync::{Arc, OnceLock};

use super::{
    Atom, Comment, Node, NodeCommentContainer, NodeId, NodeKind, NodeList, NodeStore, ScopeData,
    TextSpan, Visitor, dump, walk,
};
use crate::parser::ChunkData;
use crate::position::{ColumnSettings, LineIndex, Span};
use crate::syntax::Chunk;
use crate::syntax::greybel::{TypeSyntax, ValueExpansions};

pub struct Ast {
    nodes: NodeStore,
    root: NodeId,
    pub literals: Vec<NodeId>,
    pub comments: Vec<Comment>,
    pub scopes: Vec<NodeId>,
    line_items: Vec<NodeId>,
    extents: OnceLock<Vec<Span>>,
    source: Arc<str>,
    lines: Arc<OnceLock<LineIndex>>,
    pub imports: Vec<NodeId>,
    pub includes: Vec<NodeId>,
    pub injects: Vec<NodeId>,
    pub native_imports: Vec<NodeId>,
    pub debuggers: Vec<NodeId>,
    pub expansions: ValueExpansions,
    pub type_syntax: TypeSyntax,
}

impl Ast {
    pub(crate) fn new(
        nodes: NodeStore,
        root: NodeId,
        chunk_data: ChunkData,
        source: Arc<str>,
        lines: Arc<OnceLock<LineIndex>>,
    ) -> Self {
        Self {
            nodes,
            root,
            literals: chunk_data.literals,
            comments: chunk_data.comments,
            scopes: chunk_data.scopes,
            line_items: chunk_data.line_items,
            extents: OnceLock::new(),
            source,
            lines,
            imports: chunk_data.imports,
            includes: chunk_data.includes,
            injects: chunk_data.injects,
            native_imports: chunk_data.native_imports,
            debuggers: chunk_data.debuggers,
            expansions: ValueExpansions::default(),
            type_syntax: TypeSyntax::default(),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn lines(&self) -> &LineIndex {
        self.lines.get_or_init(|| LineIndex::new(&self.source))
    }

    pub fn resolve(&self, atom: Atom) -> &str {
        self.nodes.resolve(atom)
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn nodes(&self) -> &NodeStore {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> &Node {
        self.nodes.get(id)
    }

    #[must_use]
    pub fn node_comments(&self, id: NodeId) -> Option<&NodeCommentContainer> {
        self.nodes.comments(id)
    }

    pub fn list(&self, list: NodeList) -> &[NodeId] {
        self.nodes.list(list)
    }

    #[must_use]
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes.children(id)
    }

    pub fn for_each_child(&self, id: NodeId, f: impl FnMut(NodeId)) {
        self.nodes.for_each_child(id, f);
    }

    #[must_use]
    pub fn kind(&self, id: NodeId) -> &NodeKind {
        &self.nodes.get(id).kind
    }

    pub fn span(&self, id: NodeId) -> Span {
        self.nodes.get(id).span
    }

    pub fn text(&self, id: NodeId) -> Option<&str> {
        self.span(id).bytes().map(|bytes| &self.source[bytes])
    }

    #[must_use]
    pub fn slice(&self, span: Span) -> &str {
        span.bytes().map_or("", |bytes| &self.source[bytes])
    }

    #[must_use]
    pub fn read(&self, text: TextSpan) -> Cow<'_, str> {
        text.read(&self.source)
    }

    #[must_use]
    pub fn name(&self, id: NodeId) -> Option<&str> {
        match self.kind(id) {
            NodeKind::Identifier(identifier) => Some(self.resolve(identifier.name)),
            NodeKind::Member(member) => self.name(member.identifier),
            _ => None,
        }
    }

    fn chunk(&self) -> &Chunk {
        match self.kind(self.root) {
            NodeKind::Chunk(chunk) => chunk,
            _ => unreachable!("root is always a chunk"),
        }
    }

    pub fn body(&self) -> &[NodeId] {
        self.list(self.chunk().body)
    }

    pub fn scope(&self) -> &ScopeData {
        &self.chunk().scope
    }

    pub fn line_items(&self) -> &[NodeId] {
        &self.line_items
    }

    pub fn path_at(&self, offset: u32) -> Vec<NodeId> {
        let mut path = vec![self.root];
        let mut current = self.root;

        loop {
            let mut child_with_end_at_offset = None;
            let mut inside = None;

            for child in self.nodes.children(current) {
                let span = self.span(child);

                if span.start > offset || offset > span.end {
                    continue;
                }

                if offset < span.end {
                    inside = Some(child);
                    break;
                }

                child_with_end_at_offset.get_or_insert(child);
            }

            if let Some(child) = inside.or(child_with_end_at_offset) {
                path.push(child);
                current = child;

                continue;
            }

            if !self.push_descendant_at(current, offset, &mut path) {
                return path;
            }

            current = *path.last().expect("a pushed descendant");
        }
    }

    fn push_descendant_at(&self, id: NodeId, offset: u32, path: &mut Vec<NodeId>) -> bool {
        for child in self.nodes.children(id) {
            let extent = self.extent(child);

            if offset < extent.start || extent.end < offset || extent.is_empty() {
                continue;
            }

            path.push(child);

            let span = self.span(child);

            if (span.start <= offset && offset <= span.end && !span.is_empty())
                || self.push_descendant_at(child, offset, path)
            {
                return true;
            }

            path.pop();
        }

        false
    }

    fn extent(&self, id: NodeId) -> Span {
        let extents = self.extents.get_or_init(|| {
            let mut extents = vec![Span::default(); self.nodes.len()];

            self.collect_extent(self.root, &mut extents);

            extents
        });

        extents.get(id.index()).copied().unwrap_or_default()
    }

    fn collect_extent(&self, id: NodeId, extents: &mut [Span]) -> Span {
        let span = self.span(id);
        let mut extent = if span.is_closed() {
            span
        } else {
            Span::empty(span.start)
        };

        for child in self.nodes.children(id) {
            let child = self.collect_extent(child, extents);

            if child.is_empty() {
                continue;
            }

            if extent.is_empty() {
                extent = child;
            } else {
                extent = Span::new(extent.start.min(child.start), extent.end.max(child.end));
            }
        }

        if let Some(slot) = extents.get_mut(id.index()) {
            *slot = extent;
        }

        extent
    }

    pub fn walk(&self, id: NodeId, visitor: &mut impl Visitor) {
        walk(&self.nodes, id, visitor);
    }

    pub fn dump(&self, id: NodeId, columns: ColumnSettings) -> String {
        dump::render(&self.nodes, &self.source, self.lines(), columns, id)
    }
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "Debug output intentionally summarizes nodes and omits source text and line tables"
)]
impl fmt::Debug for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ast")
            .field("root", &self.root)
            .field("nodes", &self.nodes.len())
            .field("imports", &self.imports)
            .field("includes", &self.includes)
            .field("injects", &self.injects)
            .field("native_imports", &self.native_imports)
            .field("debuggers", &self.debuggers)
            .finish()
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.dump(self.root, ColumnSettings::default()))
    }
}
