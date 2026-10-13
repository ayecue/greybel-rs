use super::if_statement::{ElseClause, If, IfClause, IfClauseKind};
use super::loop_jump::parse_loop_jump;
use super::{Return, parse_expression_statement};
use crate::ast::{NodeId, NodeList};
use crate::diagnostics::{BlockDiagnostic, ExpectationDiagnostic, KeywordDiagnostic};
use crate::error::ParseResult;
use crate::lexer::{KEYWORDS, Keyword, Punct, TokenKind};
use crate::parser::{Expect, Parser, groups};
use crate::position::Span;
use crate::syntax::Empty;

pub fn parse_shortcut_statement(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
    let Some(keyword) = parser.token().keyword() else {
        return parse_expression_statement(parser);
    };

    match keyword {
        Keyword::Not => parse_expression_statement(parser),
        Keyword::Return => {
            parser.advance()?;

            Return::parse(parser)
        }
        Keyword::Continue | Keyword::Break => {
            parser.advance()?;

            parse_loop_jump(parser, keyword)
        }
        Keyword::If => {
            parser.advance()?;

            parse_inner_if(parser)
        }
        Keyword::Else => {
            let span = parser.token().span();

            parser.advance()?;
            if !parser.is_at_any(groups::STATEMENT_END) {
                let diagnostic = ExpectationDiagnostic::OneOf {
                    got: parser.token().describe(parser.source()),
                    expectations: vec![
                        Expect::Keyword(Keyword::Else).describe(),
                        Expect::Eol.describe(),
                    ],
                };
                let span = parser.token().span();

                parser.raise_at(diagnostic, span)?;

                return parser.parse_invalid_code();
            }

            Ok(parser.alloc(Empty, span))
        }
        _ => {
            let span = parser.token().span();

            match keyword {
                Keyword::For | Keyword::While
                    if parser.grammar().explains_single_line_if_errors =>
                {
                    parser.raise_at(KeywordDiagnostic::LoopInSingleLineIf, span)?;
                }
                Keyword::For | Keyword::While => parser.raise_at(
                    BlockDiagnostic::UnmatchedEnd {
                        end: Keyword::EndIf.as_str().to_owned(),
                        block: Keyword::If.as_str(),
                    },
                    span,
                )?,
                Keyword::EndIf | Keyword::EndFor | Keyword::EndWhile => {
                    let block = match keyword {
                        Keyword::EndIf => Keyword::If,
                        Keyword::EndFor => Keyword::For,
                        _ => Keyword::While,
                    };

                    parser.raise_at(
                        BlockDiagnostic::UnmatchedEnd {
                            end: keyword.as_str().to_owned(),
                            block: block.as_str(),
                        },
                        span,
                    )?;
                }
                Keyword::EndFunction => parser.raise_at(
                    BlockDiagnostic::WithoutStarter(Keyword::EndFunction.as_str()),
                    span,
                )?,
                _ => parser.raise_at(KeywordDiagnostic::Misplacement(keyword), span)?,
            }

            parser.parse_invalid_code()
        }
    }
}

pub fn is_at_clause_end(parser: &Parser<'_>) -> bool {
    parser.token().is_keyword(Keyword::Else)
        || (parser.grammar().single_line_else_if && parser.token().is_keyword(Keyword::ElseIf))
}

pub(super) fn parse_if_shortcut(
    parser: &mut Parser<'_>,
    condition: NodeId,
    start: Span,
) -> ParseResult<()> {
    let depth = parser.stream.blocks.depth();
    let (node, mut line_continues) = parse_inner_if_shortcut(parser, condition, start)?;

    parser.emit_statement(depth, node, true);
    if !line_continues && is_dropped_keyword_or_literal(parser) {
        parser.advance()?;
        line_continues = true;
    }

    parser.stream.line_continues = line_continues;

    Ok(())
}

fn parse_inner_if(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
    let start = parser.prev().span();
    let condition = parser.parse_expr(None)?;

    parser.register_line_item(condition);
    require_then(parser, start)?;

    parse_inner_if_shortcut(parser, condition, start).map(|(node, _)| node)
}

fn parse_inner_if_shortcut(
    parser: &mut Parser<'_>,
    condition: NodeId,
    start: Span,
) -> ParseResult<(NodeId, bool)> {
    let node = parser.alloc(
        If {
            shortcut: true,
            clauses: NodeList::EMPTY,
        },
        start.open(),
    );

    let depth = parser.stream.blocks.depth();
    let item = parse_shortcut_statement(parser)?;

    if !parser.grammar().functions_in_single_line_if && parser.stream.blocks.depth() > depth {
        let span = parser.token().span();

        parser.raise_at(
            BlockDiagnostic::UnmatchedEnd {
                end: Keyword::EndIf.as_str().to_owned(),
                block: Keyword::If.as_str(),
            },
            span,
        )?;
    }

    let body = parser.stream.nodes.push_list(&[item]);
    let first = parser.alloc(
        IfClause {
            kind: IfClauseKind::If,
            shortcut: true,
            condition,
            body,
        },
        start.to(&parser.token().span()),
    );
    let mut clauses = parser.stream.nodes.list_buffer();

    clauses.push(first);

    while parser.grammar().single_line_else_if && parser.token().is_keyword(Keyword::ElseIf) {
        parser.advance()?;

        let clause_start = parser.prev().span();
        let condition = parser.parse_expr(None)?;

        parser.register_line_item(condition);
        require_then(parser, clause_start)?;

        let item = parse_shortcut_statement(parser)?;
        let body = parser.stream.nodes.push_list(&[item]);

        clauses.push(parser.alloc(
            IfClause {
                kind: IfClauseKind::ElseIf,
                shortcut: true,
                condition,
                body,
            },
            clause_start.to(&parser.token().span()),
        ));
    }

    let mut line_continues = false;

    if parser.token().is_keyword(Keyword::Else) {
        parser.advance()?;

        let else_start = parser.token().span();
        let item = parse_shortcut_statement(parser)?;
        let body = parser.stream.nodes.push_list(&[item]);

        clauses.push(parser.alloc(
            ElseClause {
                shortcut: true,
                body,
            },
            else_start.to(&parser.token().span()),
        ));
        line_continues = true;
    }

    let end = parser.token().span();
    let clauses = parser.stream.nodes.finish_list(clauses);

    parser.stream.nodes.close(node, &end);
    parser.stream.nodes.set_kind(
        node,
        If {
            shortcut: true,
            clauses,
        },
    );

    Ok((node, line_continues))
}

pub(super) fn require_then(parser: &mut Parser<'_>, start: Span) -> ParseResult<bool> {
    if parser.grammar().explains_single_line_if_errors && parser.token().is_punct(Punct::Assign) {
        let span = parser.token().span();

        parser.raise_at(KeywordDiagnostic::AssignmentInCondition, span)?;
    }

    parser.require(Expect::Keyword(Keyword::Then), Some(start.start))
}

fn is_dropped_keyword_or_literal(parser: &Parser<'_>) -> bool {
    match parser.token().kind() {
        TokenKind::BooleanLiteral | TokenKind::NilLiteral => true,
        TokenKind::Keyword => parser
            .token()
            .keyword()
            .is_some_and(|k| KEYWORDS.contains(&k)),
        _ => false,
    }
}
