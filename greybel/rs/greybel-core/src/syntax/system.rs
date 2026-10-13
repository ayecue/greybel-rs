use crate::ast::{AstNode, DumpContext, NodeFieldContainer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, NodeFieldContainer)]
pub struct Empty;

impl AstNode for Empty {
    fn type_name(&self) -> &'static str {
        "EmptyExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!("{}[{}][]", self.type_name(), ctx.location())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, NodeFieldContainer)]
pub struct Unknown;

impl AstNode for Unknown {
    fn type_name(&self) -> &'static str {
        "Unknown"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!("{}[{}][]", self.type_name(), ctx.location())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, NodeFieldContainer)]
pub struct Invalid;

impl AstNode for Invalid {
    fn type_name(&self) -> &'static str {
        "InvalidCodeExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!("{}[{}][]", self.type_name(), ctx.location())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, NodeFieldContainer)]
pub struct Noop;

impl AstNode for Noop {
    fn type_name(&self) -> &'static str {
        "NoopStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!("{}[{}][]", self.type_name(), ctx.location())
    }
}
