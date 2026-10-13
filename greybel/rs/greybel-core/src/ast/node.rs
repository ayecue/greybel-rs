use std::fmt;
use std::num::NonZeroU32;

use super::{DumpContext, NodeList, NodeListStorage, Scope};
use crate::position::Span;
use crate::syntax::greybel::{Bitwise, Debugger, Envar, File, Import, Include, Inject, Line};
use crate::syntax::greyscript::ImportCode;
use crate::syntax::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(NonZeroU32);

impl NodeId {
    pub(crate) fn from_index(index: usize) -> Self {
        u32::try_from(index)
            .ok()
            .and_then(|index| NonZeroU32::new(index.wrapping_add(1)))
            .map(NodeId)
            .expect("too many AST nodes")
    }

    #[must_use]
    pub fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NodeId").field(&self.index()).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Child {
    Node(NodeId),
    List(NodeList),
}

pub trait NodeFieldContainer {
    fn children(&self, lists: &NodeListStorage) -> Vec<NodeId>;

    fn map_children(&mut self, f: &mut dyn FnMut(Child) -> Child);

    fn for_each_child(&self, lists: &NodeListStorage, f: &mut impl FnMut(NodeId))
    where
        Self: Sized;

    fn as_block(&self) -> Option<&dyn Block> {
        None
    }

    fn as_block_mut(&mut self) -> Option<&mut dyn Block> {
        None
    }

    fn as_scope(&self) -> Option<&dyn Scope> {
        None
    }

    fn as_scope_mut(&mut self) -> Option<&mut dyn Scope> {
        None
    }
}

pub trait AstNode: NodeFieldContainer {
    fn type_name(&self) -> &'static str;
    fn dump(&self, ctx: &DumpContext<'_>) -> String;
}

pub trait Block {
    fn body(&self) -> NodeList;
    fn set_body(&mut self, body: NodeList);
}

pub trait ChildField {
    fn collect(&self, lists: &NodeListStorage, out: &mut Vec<NodeId>);
    fn visit(&self, lists: &NodeListStorage, f: &mut impl FnMut(NodeId));
    fn map(&mut self, f: &mut dyn FnMut(Child) -> Child);
}

impl ChildField for NodeId {
    fn collect(&self, _lists: &NodeListStorage, out: &mut Vec<NodeId>) {
        out.push(*self);
    }

    fn visit(&self, _lists: &NodeListStorage, f: &mut impl FnMut(NodeId)) {
        f(*self);
    }

    fn map(&mut self, f: &mut dyn FnMut(Child) -> Child) {
        if let Child::Node(id) = f(Child::Node(*self)) {
            *self = id;
        }
    }
}

impl ChildField for Option<NodeId> {
    fn collect(&self, _lists: &NodeListStorage, out: &mut Vec<NodeId>) {
        out.extend(*self);
    }

    fn visit(&self, _lists: &NodeListStorage, f: &mut impl FnMut(NodeId)) {
        if let Some(id) = self {
            f(*id);
        }
    }

    fn map(&mut self, f: &mut dyn FnMut(Child) -> Child) {
        if let Some(id) = self {
            id.map(f);
        }
    }
}

impl ChildField for NodeList {
    fn collect(&self, lists: &NodeListStorage, out: &mut Vec<NodeId>) {
        out.extend_from_slice(lists.get(*self));
    }

    fn visit(&self, lists: &NodeListStorage, f: &mut impl FnMut(NodeId)) {
        for &id in lists.get(*self) {
            f(id);
        }
    }

    fn map(&mut self, f: &mut dyn FnMut(Child) -> Child) {
        if let Child::List(list) = f(Child::List(*self)) {
            *self = list;
        }
    }
}

macro_rules! node_kinds {
    (
        inline: [$($inline:ident),* $(,)?],
        boxed: [$($boxed:ident),* $(,)?] $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum NodeKind {
            $($inline($inline),)*
            $($boxed(Box<$boxed>),)*
        }

        $(
            impl From<$inline> for NodeKind {
                fn from(node: $inline) -> Self {
                    NodeKind::$inline(node)
                }
            }
        )*

        $(
            impl From<$boxed> for NodeKind {
                fn from(node: $boxed) -> Self {
                    NodeKind::$boxed(Box::new(node))
                }
            }
        )*

        impl NodeKind {
            pub fn as_node(&self) -> &dyn AstNode {
                match self {
                    $(NodeKind::$inline(node) => node,)*
                    $(NodeKind::$boxed(node) => &**node,)*
                }
            }

            pub fn as_node_mut(&mut self) -> &mut dyn AstNode {
                match self {
                    $(NodeKind::$inline(node) => node,)*
                    $(NodeKind::$boxed(node) => &mut **node,)*
                }
            }

            fn visit_children(&self, lists: &NodeListStorage, f: &mut impl FnMut(NodeId)) {
                match self {
                    $(NodeKind::$inline(node) => node.for_each_child(lists, f),)*
                    $(NodeKind::$boxed(node) => node.for_each_child(lists, f),)*
                }
            }
        }
    };
}

node_kinds! {
    inline: [
        Assignment,
        CallStatement,
        Return,
        If,
        IfClause,
        ElseClause,
        While,
        For,
        Break,
        Continue,
        Identifier,
        Member,
        Index,
        Slice,
        Call,
        Literal,
        Unary,
        Binary,
        Logical,
        Isa,
        Map,
        MapEntry,
        List,
        ListItem,
        Parenthesis,
        Empty,
        Unknown,
        Invalid,
        Noop,
        Import,
        Include,
        Envar,
        Inject,
        Debugger,
        Line,
        File,
        Bitwise,
        ImportCode,
    ],
    boxed: [
        Chunk,
        Function,
        ComparisonGroup,
    ],
}

impl NodeFieldContainer for NodeKind {
    fn children(&self, lists: &NodeListStorage) -> Vec<NodeId> {
        self.as_node().children(lists)
    }

    fn map_children(&mut self, f: &mut dyn FnMut(Child) -> Child) {
        self.as_node_mut().map_children(f);
    }

    fn for_each_child(&self, lists: &NodeListStorage, f: &mut impl FnMut(NodeId)) {
        self.visit_children(lists, f);
    }

    fn as_block(&self) -> Option<&dyn Block> {
        self.as_node().as_block()
    }

    fn as_block_mut(&mut self) -> Option<&mut dyn Block> {
        self.as_node_mut().as_block_mut()
    }

    fn as_scope(&self) -> Option<&dyn Scope> {
        self.as_node().as_scope()
    }

    fn as_scope_mut(&mut self) -> Option<&mut dyn Scope> {
        self.as_node_mut().as_scope_mut()
    }
}

impl AstNode for NodeKind {
    fn type_name(&self) -> &'static str {
        self.as_node().type_name()
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        self.as_node().dump(ctx)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    pub span: Span,
    pub scope: Option<NodeId>,
    pub(crate) depth: u16,
}

impl Node {
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(self.kind, NodeKind::Literal(_))
    }
}
