use super::state::{
    BlockSnapshot, BlockStack, ChunkDataSnapshot, ChunkDataStack, CommentAttachment,
    ParseErrorStorage, ScopeStack, ScopeStackSnapshot,
};
use crate::ast::{Ast, NodeId, NodeStore};
use crate::lexer::{Lexer, TokenSnapshot, TokenStream};
use crate::position::Span;
use crate::syntax::greybel::{TypeSyntax, ValueExpansions};

pub struct ParseStream {
    pub(crate) tokens: TokenStream,
    pub(crate) nodes: NodeStore,
    pub(crate) scopes: ScopeStack,
    pub(crate) blocks: BlockStack,
    pub(crate) chunk_data: ChunkDataStack,
    pub(crate) comments: CommentAttachment,
    pub(crate) diagnostics: ParseErrorStorage,
    pub(crate) root: Option<NodeId>,
    pub(crate) refused_operator: Option<u32>,
    pub(crate) line_continues: bool,
    pub(crate) expression_depth: usize,
    pub(crate) over_depth_span: Option<Span>,
    pub(crate) statement: Option<u32>,
    pub(crate) expands_values: bool,
    pub(crate) type_syntax: TypeSyntax,
}

pub struct ParseSnapshot {
    tokens: TokenSnapshot,
    scopes: ScopeStackSnapshot,
    blocks: BlockSnapshot,
    chunk_data: ChunkDataSnapshot,
    refused_operator: Option<u32>,
}

impl Default for ParseStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ParseStream {
    #[must_use]
    pub fn new() -> Self {
        Self::with(TokenStream::new(), ParseErrorStorage::default())
    }

    #[must_use]
    pub fn new_unsafe() -> Self {
        Self::with(TokenStream::new_unsafe(), ParseErrorStorage::new_unsafe())
    }

    fn with(tokens: TokenStream, diagnostics: ParseErrorStorage) -> Self {
        Self {
            tokens,
            nodes: NodeStore::default(),
            scopes: ScopeStack::default(),
            blocks: BlockStack::default(),
            chunk_data: ChunkDataStack::default(),
            comments: CommentAttachment::default(),
            diagnostics,
            root: None,
            refused_operator: None,
            line_continues: false,
            expands_values: false,
            type_syntax: TypeSyntax::default(),
            expression_depth: 0,
            over_depth_span: None,
            statement: None,
        }
    }

    #[must_use]
    pub fn tokens(&self) -> &TokenStream {
        &self.tokens
    }

    #[must_use]
    pub fn nodes(&self) -> &NodeStore {
        &self.nodes
    }

    #[must_use]
    pub fn scopes(&self) -> &ScopeStack {
        &self.scopes
    }

    #[must_use]
    pub fn blocks(&self) -> &BlockStack {
        &self.blocks
    }

    pub fn blocks_mut(&mut self) -> &mut BlockStack {
        &mut self.blocks
    }

    pub fn chunk_data_mut(&mut self) -> &mut ChunkDataStack {
        &mut self.chunk_data
    }

    #[must_use]
    pub fn is_unsafe(&self) -> bool {
        self.diagnostics.is_unsafe()
    }

    pub fn snapshot(&mut self) -> ParseSnapshot {
        ParseSnapshot {
            tokens: self.tokens.snapshot(),
            scopes: self.scopes.snapshot(),
            blocks: self.blocks.snapshot(),
            chunk_data: self.chunk_data.snapshot(),
            refused_operator: self.refused_operator,
        }
    }

    pub fn restore(&mut self, snapshot: ParseSnapshot) {
        self.tokens.restore(snapshot.tokens);
        self.chunk_data.restore(snapshot.chunk_data);
        self.scopes.restore(snapshot.scopes, &mut self.nodes);
        self.blocks.restore(snapshot.blocks);
        self.refused_operator = snapshot.refused_operator;
        self.line_continues = false;
        self.over_depth_span = None;
    }

    pub(crate) fn into_ast(self, lexer: &Lexer<'_>) -> Ast {
        let root = self.root.expect("chunk was parsed");
        let chunk_data = self.chunk_data.into_base();
        let mut nodes = self.nodes;
        let expansions = if self.expands_values {
            ValueExpansions::of(&mut nodes, root)
        } else {
            ValueExpansions::default()
        };
        let mut ast = Ast::new(
            nodes,
            root,
            chunk_data,
            lexer.source().into(),
            lexer.shared_lines(),
        );

        ast.expansions = expansions;
        ast.type_syntax = self.type_syntax;

        ast
    }
}
