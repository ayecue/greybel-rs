use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind};
use crate::dialect::SyntaxFeatures;
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::Parser;

/// `...items` in a list or map literal. Builds write the literal as additions: `[...a, 1]` becomes `[] + a + [1]`.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Spread {
    #[child]
    pub argument: NodeId,
}

impl AstNode for Spread {
    fn type_name(&self) -> &'static str {
        "SpreadExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "SpreadExpression[{}][{}]",
            ctx.location(),
            ctx.child(self.argument)
        )
    }
}

impl Spread {
    /// A spread when the current token is `...` and the grammar has Greybel's syntax features.
    pub(crate) fn parse(parser: &mut Parser<'_>, item: NodeId) -> ParseResult<Option<NodeKind>> {
        if !(parser.grammar().allows(SyntaxFeatures::SPREAD)
            && parser.token().is_punct(Punct::Ellipsis))
        {
            return Ok(None);
        }

        parser.advance()?;

        let argument = parser.parse_expr(Some(item))?;

        Ok(Some(Spread { argument }.into()))
    }
}
