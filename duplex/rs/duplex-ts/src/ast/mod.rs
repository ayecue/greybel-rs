mod classes;
mod comments;
mod declarations;
mod expressions;
mod statements;
mod types;

use std::cell::RefCell;

pub(crate) use classes::{ModifierContainer, Parameter, TypeParameter};
use comments::CommentStorage;
pub(crate) use declarations::{EnumMember, Import, InterfaceProperty};
use duplex_protocol::DocumentationRef;
use oxc_allocator::{Allocator, ArenaVec, GetAllocator};
use oxc_ast::ast::{Program, Statement};
use oxc_ast::builder::{AstBuilder, GetAstBuilder};
use oxc_codegen::{Codegen, CodegenOptions, IndentChar};
use oxc_span::{SPAN, SourceType, Span};

const INDENT_WIDTH: usize = 2;

pub(crate) struct Ts<'a> {
    builder: AstBuilder<'a>,
    comments: RefCell<CommentStorage>,
}

impl<'a> GetAstBuilder<'a> for Ts<'a> {
    type Builder = AstBuilder<'a>;

    fn builder(&self) -> &AstBuilder<'a> {
        &self.builder
    }
}

impl<'a> GetAllocator<'a> for Ts<'a> {
    fn allocator(&self) -> &'a Allocator {
        self.builder.allocator()
    }
}

impl<'a> Ts<'a> {
    pub(crate) fn new(allocator: &'a Allocator) -> Self {
        Self {
            builder: AstBuilder::new(allocator),
            comments: RefCell::default(),
        }
    }

    pub(crate) fn text(&self, text: &str) -> &'a str {
        self.allocator().alloc_str(text)
    }

    pub(crate) fn vec<T>(&self, items: impl IntoIterator<Item = T>) -> ArenaVec<'a, T> {
        ArenaVec::from_iter_in(items, self)
    }

    pub(crate) fn docs(&self, docs: &DocumentationRef) -> Span {
        self.comments.borrow_mut().document(docs.lines())
    }

    pub(crate) fn doc(&self, line: String) -> Span {
        self.comments.borrow_mut().document(&[line])
    }

    pub(crate) fn header(&self, line: &str) {
        self.comments.borrow_mut().header(line);
    }

    pub(crate) fn print(&self, body: Vec<Statement<'a>>) -> String {
        let (text, comments) = self.comments.take().into_parts();
        let program = Program::new(
            SPAN,
            SourceType::ts(),
            self.text(&text),
            self.vec(comments),
            None,
            self.vec([]),
            self.vec(body),
            self,
        );

        Codegen::new()
            .with_options(CodegenOptions {
                indent_char: IndentChar::Space,
                indent_width: INDENT_WIDTH,
                ..CodegenOptions::default()
            })
            .build(&program)
            .code
    }
}
