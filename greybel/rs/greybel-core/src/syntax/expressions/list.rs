use super::Spread;
use super::collection::CollectionLiteral;
use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeList};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{ExprContext, Parser, PrefixRule, Trigger, groups};

#[derive(Debug, Clone, Default, PartialEq, NodeFieldContainer)]
pub struct List {
    #[child]
    pub fields: NodeList,
}

impl AstNode for List {
    fn type_name(&self) -> &'static str {
        "ListConstructorExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let loc = ctx.location();

        if self.fields.is_empty() {
            format!("ListConstructor[{loc}][]")
        } else {
            format!("ListConstructor[{loc}][\n{}\n]", ctx.block(self.fields))
        }
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct ListItem {
    #[child]
    pub value: NodeId,
}

impl AstNode for ListItem {
    fn type_name(&self) -> &'static str {
        "ListValue"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!("{type_name}[{}][{}]", ctx.location(), ctx.child(self.value))
    }
}

const LIST: CollectionLiteral = CollectionLiteral {
    close: Punct::RBracket,
    separators: groups::LIST_SEPARATOR,
};

#[derive(Debug, Clone, Copy)]
pub struct ListRule;

impl PrefixRule for ListRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::LBracket)]
    }

    fn parse(&self, parser: &mut Parser<'_>, _context: ExprContext) -> ParseResult<NodeId> {
        let list = parser.alloc(List::default(), parser.token().span().open());
        let fields = LIST.parse(parser, list, |parser, item| {
            if let Some(spread) = Spread::parse(parser, item)? {
                return Ok(spread);
            }

            Ok(ListItem {
                value: parser.parse_expr(Some(item))?,
            }
            .into())
        })?;

        parser.stream.nodes.set_kind(list, List { fields });

        Ok(list)
    }
}
