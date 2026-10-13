use crate::ast::{NodeId, NodeKind, NodeList};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{Expect, Parser};
use crate::syntax::Invalid;

pub(crate) struct CollectionLiteral {
    pub close: Punct,
    pub separators: &'static [Expect],
}

impl CollectionLiteral {
    pub fn parse<K: Into<NodeKind>>(
        &self,
        parser: &mut Parser<'_>,
        container: NodeId,
        mut parse_item: impl FnMut(&mut Parser<'_>, NodeId) -> ParseResult<K>,
    ) -> ParseResult<NodeList> {
        let start = parser.token().span();
        let mut items = parser.stream.nodes.list_buffer();

        parser.advance()?;
        if parser.token().is_punct(self.close) {
            parser.advance()?;
        } else {
            parser.consume_noop()?;
            while !parser.token().is_eof() {
                if parser.token().is_punct(self.close) {
                    parser.advance()?;
                    break;
                }

                let item = parser.alloc(Invalid, parser.token().span().open());
                let kind = parse_item(parser, item)?;
                let end = parser.prev().span();

                parser.stream.nodes.close(item, &end);
                parser.stream.nodes.set_kind(item, kind);
                items.push(item);

                if parser.grammar().relaxed_separators {
                    parser.consume_inline_comment(item)?;
                    parser.consume_noop()?;
                }

                if parser.require_any(self.separators, Some(start.start))?
                    && parser.prev().is_punct(self.close)
                {
                    break;
                }

                parser.consume_inline_comment(item)?;
                parser.consume_noop()?;
            }
        }

        let end = parser.token().span().start;

        parser.stream.nodes.get_mut(container).span.end = end;

        Ok(parser.stream.nodes.finish_list(items))
    }
}
