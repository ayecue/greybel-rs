use crate::ast::NodeFieldContainer;
use crate::lexer::Keyword;
use crate::parser::{ExprContext, PrefixRule, Trigger};
use crate::{AstNode, DumpContext, NodeId, ParseResult, Parser};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct File {
    pub filename: String,
}

impl AstNode for File {
    fn type_name(&self) -> &'static str {
        "FeatureFileExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!("{type_name}[{}][path = {}]", ctx.location(), self.filename)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FileRule;

impl PrefixRule for FileRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Keyword(Keyword::Filename)]
    }

    fn parse(&self, parser: &mut Parser<'_>, _context: ExprContext) -> ParseResult<NodeId> {
        parser.advance()?;

        let filename = parser.options().filename.clone();

        Ok(parser.alloc(File { filename }, parser.prev().span()))
    }
}
