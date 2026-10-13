use super::shortcut::{parse_if_shortcut, require_then};
use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList, NodeStore};
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{KeywordStatement, Parser, PendingBlock, groups};
use crate::position::Span;

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct If {
    pub shortcut: bool,
    #[child]
    pub clauses: NodeList,
}

impl AstNode for If {
    fn type_name(&self) -> &'static str {
        if self.shortcut {
            "IfShortcutStatement"
        } else {
            "IfStatement"
        }
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let loc = ctx.location();

        if self.clauses.is_empty() {
            format!("IfStatement[{loc}][]")
        } else {
            format!("IfStatement[{loc}][\n{}\n]", ctx.block(self.clauses))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IfClauseKind {
    If,
    ElseIf,
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct IfClause {
    pub kind: IfClauseKind,
    pub shortcut: bool,
    #[child]
    pub condition: NodeId,
    #[child]
    #[body]
    pub body: NodeList,
}

impl AstNode for IfClause {
    fn type_name(&self) -> &'static str {
        match (self.kind, self.shortcut) {
            (IfClauseKind::If, false) => "IfClause",
            (IfClauseKind::If, true) => "IfShortcutClause",
            (IfClauseKind::ElseIf, false) => "ElseifClause",
            (IfClauseKind::ElseIf, true) => "ElseifShortcutClause",
        }
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let name = self.type_name();
        let loc = ctx.location();
        let condition = ctx.child(self.condition);

        if self.body.is_empty() {
            format!("{name}[{loc}][{condition}]")
        } else {
            format!("{name}[{loc}][{condition}\n{}\n]", ctx.block(self.body))
        }
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct ElseClause {
    pub shortcut: bool,
    #[child]
    #[body]
    pub body: NodeList,
}

impl AstNode for ElseClause {
    fn type_name(&self) -> &'static str {
        if self.shortcut {
            "ElseShortcutClause"
        } else {
            "ElseClause"
        }
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let name = self.type_name();

        if self.body.is_empty() {
            format!("{name}[]")
        } else {
            format!("{name}[{}][\n{}\n}}]", ctx.location(), ctx.block(self.body))
        }
    }
}

pub struct OpenIf {
    pub node: NodeId,
    pub clause: NodeId,
    pub body: Vec<NodeId>,
    pub clauses: Vec<NodeId>,
}

impl PendingBlock for OpenIf {
    fn body_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.body
    }

    fn name(&self) -> &'static str {
        Keyword::If.as_str()
    }

    fn awaits_end(&self, nodes: &NodeStore) -> bool {
        matches!(nodes.get(self.clause).kind, NodeKind::ElseClause(_))
    }

    fn start(&self, nodes: &NodeStore) -> u32 {
        nodes.get(self.node).span.start
    }

    fn complete(self: Box<Self>, parser: &mut Parser<'_>, end: &Span) {
        let Self {
            node,
            clause,
            body,
            mut clauses,
        } = *self;
        let nodes = &mut parser.stream.nodes;

        if !body.is_empty() {
            nodes.close_block(clause, body, end);
            clauses.push(clause);
        }

        let clauses = nodes.finish_list(clauses);

        if let NodeKind::If(node) = &mut nodes.get_mut(node).kind {
            node.clauses = clauses;
        }

        nodes.close(node, end);
        parser.register_line_item(node);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NextClause {
    ElseIf,
    Else,
    End,
}

#[derive(Debug, Clone, Copy)]
pub struct IfRule;

impl KeywordStatement for IfRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::If]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let start = parser.prev().span();
        let condition = parser.parse_expr(None)?;

        parser.register_line_item(condition);
        require_then(parser, start)?;

        if !parser.is_at_any(groups::BLOCK_END_OF_LINE) {
            return parse_if_shortcut(parser, condition, start);
        }

        let node = parser.alloc(
            If {
                shortcut: false,
                clauses: NodeList::EMPTY,
            },
            start.open(),
        );
        let clause = parser.alloc(
            IfClause {
                kind: IfClauseKind::If,
                shortcut: false,
                condition,
                body: NodeList::EMPTY,
            },
            start.to(&parser.token().span()),
        );

        parser.attach_pending_comments(node);
        parser.set_last_statement(clause);

        let (body, clauses) = (
            parser.stream.nodes.list_buffer(),
            parser.stream.nodes.list_buffer(),
        );

        parser.stream.blocks.push(OpenIf {
            node,
            clause,
            body,
            clauses,
        });

        Ok(())
    }
}

fn next_clause(parser: &mut Parser<'_>, next: NextClause) -> ParseResult<()> {
    let buffer = parser.stream.nodes.list_buffer();
    let Some(block) = parser.stream.blocks.top_mut_as::<OpenIf>() else {
        return match next {
            NextClause::End => {
                parser.raise_unmatched_end(Keyword::EndIf.as_str(), Keyword::If.as_str())
            }
            NextClause::Else | NextClause::ElseIf => {
                parser.raise_without_starter(Keyword::Else.as_str())
            }
        };
    };
    let (node, clause) = (block.node, block.clause);
    let body = std::mem::replace(&mut block.body, buffer);

    block.clauses.push(clause);

    let end = parser.prev().span();

    parser.stream.nodes.close_block(clause, body, &end);

    let new_clause = match next {
        NextClause::ElseIf => {
            let start = parser.token().span();
            let condition = parser.parse_expr(None)?;

            require_then(parser, start)?;

            parser.alloc(
                IfClause {
                    kind: IfClauseKind::ElseIf,
                    shortcut: false,
                    condition,
                    body: NodeList::EMPTY,
                },
                start.open(),
            )
        }
        NextClause::Else => {
            let start = parser.prev().span();

            parser.alloc(
                ElseClause {
                    shortcut: false,
                    body: NodeList::EMPTY,
                },
                start.open(),
            )
        }
        NextClause::End => {
            parser.complete_top_block(&end);
            parser.stream.blocks.push_statement(node);

            return Ok(());
        }
    };

    if let Some(block) = parser.stream.blocks.top_mut_as::<OpenIf>() {
        block.clause = new_clause;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct ElseRule;

impl KeywordStatement for ElseRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::ElseIf, Keyword::Else]
    }

    fn parse(&self, parser: &mut Parser<'_>, keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;
        parser.flush_pending_comments();

        let next = if keyword == Keyword::ElseIf {
            NextClause::ElseIf
        } else {
            NextClause::Else
        };

        next_clause(parser, next)?;
        if let Some(clause) = parser
            .stream
            .blocks
            .top_mut_as::<OpenIf>()
            .map(|block| block.clause)
        {
            parser.consume_inline_comment(clause)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndIfRule;

impl KeywordStatement for EndIfRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::EndIf]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.parse_block_end(|parser| next_clause(parser, NextClause::End))
    }
}
