use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::Parser;

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Assert {
    #[child]
    pub condition: NodeId,
    #[child]
    pub message: Option<NodeId>,
    pub filename: String,
}

impl AstNode for Assert {
    fn type_name(&self) -> &'static str {
        "AssertStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let message = self
            .message
            .map_or(String::new(), |message| format!(", {}", ctx.child(message)));

        format!(
            "AssertStatement[{}][{}{message}]",
            ctx.location(),
            ctx.child(self.condition)
        )
    }
}

impl Assert {
    pub const NO_ASSERT_ENVAR: &'static str = "NO_ASSERTS";

    #[must_use]
    pub fn is_removed(environment: &std::collections::HashMap<String, String>) -> bool {
        environment
            .get(Self::NO_ASSERT_ENVAR)
            .is_some_and(|value| !matches!(value.as_str(), "" | "0" | "false"))
    }

    #[must_use]
    pub fn default_message(filename: &str, line: u32) -> String {
        let name = filename.rsplit(['/', '\\']).next().unwrap_or_default();

        format!("Assertion failed at {name}:{line}")
    }

    pub(crate) fn parse(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let start = parser.token().span();

        parser.advance()?;

        let condition = parser.parse_expr(None)?;
        let message = if parser.token().is_punct(Punct::Comma) {
            parser.advance()?;

            Some(parser.parse_expr(None)?)
        } else {
            None
        };
        let filename = parser.options().filename.clone();
        let span = parser.span_from(&start);

        Ok(parser.alloc(
            Assert {
                condition,
                message,
                filename,
            },
            span,
        ))
    }
}
