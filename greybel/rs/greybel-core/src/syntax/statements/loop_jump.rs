use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId};
use crate::diagnostics::KeywordDiagnostic;
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{KeywordStatement, Parser};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Break {
    pub loop_node: Option<NodeId>,
}

impl AstNode for Break {
    fn type_name(&self) -> &'static str {
        "BreakStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!("{type_name}[{}]", ctx.location())
    }
}

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Continue {
    pub loop_node: Option<NodeId>,
}

impl AstNode for Continue {
    fn type_name(&self) -> &'static str {
        "ContinueStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();

        format!("{type_name}[{}]", ctx.location())
    }
}

pub fn parse_loop_jump(parser: &mut Parser<'_>, keyword: Keyword) -> ParseResult<NodeId> {
    let is_break = keyword == Keyword::Break;
    let loop_node = parser.stream.scopes.innermost_loop();

    if loop_node.is_none() {
        let span = parser.prev().span();

        let diagnostic = if is_break && parser.grammar().break_outside_loop_is_unmatched_block {
            KeywordDiagnostic::UnmatchedBreak
        } else {
            KeywordDiagnostic::JumpOutsideLoop(keyword)
        };

        parser.report_at(diagnostic, span)?;
    }

    let span = parser.prev().span();

    Ok(if is_break {
        parser.alloc(Break { loop_node }, span)
    } else {
        parser.alloc(Continue { loop_node }, span)
    })
}

#[derive(Debug, Clone, Copy)]
pub struct LoopJumpRule;

impl KeywordStatement for LoopJumpRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::Break, Keyword::Continue]
    }

    fn parse(&self, parser: &mut Parser<'_>, keyword: Keyword) -> ParseResult<()> {
        let depth = parser.stream.blocks.depth();

        parser.advance()?;

        let item = parse_loop_jump(parser, keyword)?;

        parser.emit_statement(depth, item, true);

        Ok(())
    }
}
