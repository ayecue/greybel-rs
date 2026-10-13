use super::collection::CollectionLiteral;
use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeList};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{Expect, ExprContext, Parser, PrefixRule, Trigger, groups};

#[derive(Debug, Clone, Default, PartialEq, NodeFieldContainer)]
pub struct Map {
    #[child]
    pub fields: NodeList,
}

impl AstNode for Map {
    fn type_name(&self) -> &'static str {
        "MapConstructorExpression"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let loc = ctx.location();

        if self.fields.is_empty() {
            format!("MapConstructor[{loc}][]")
        } else {
            format!("MapConstructor[{loc}][\n{}\n]", ctx.block(self.fields))
        }
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct MapEntry {
    #[child]
    pub key: NodeId,
    #[child]
    pub value: NodeId,
}

impl AstNode for MapEntry {
    fn type_name(&self) -> &'static str {
        "MapKeyString"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!(
            "{type_name}[{}][{}: {}]",
            ctx.location(),
            ctx.child(self.key),
            ctx.child(self.value)
        )
    }
}

const MAP: CollectionLiteral = CollectionLiteral {
    close: Punct::RBrace,
    separators: groups::MAP_SEPARATOR,
};

#[derive(Debug, Clone, Copy)]
pub struct MapRule;

impl PrefixRule for MapRule {
    fn triggers(&self) -> &[Trigger] {
        &[Trigger::Punct(Punct::LBrace)]
    }

    fn parse(&self, parser: &mut Parser<'_>, _context: ExprContext) -> ParseResult<NodeId> {
        let map = parser.alloc(Map::default(), parser.token().span().open());
        let fields = MAP.parse(parser, map, |parser, entry| {
            let key = parser.parse_expr(None)?;

            parser.require(Expect::SliceOperator, None)?;
            parser.consume_noop()?;

            let value = parser.parse_expr(Some(entry))?;

            Ok(MapEntry { key, value })
        })?;

        parser.stream.nodes.set_kind(map, Map { fields });

        Ok(map)
    }
}
