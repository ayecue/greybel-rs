mod env_condition;
mod envar;
mod file;
mod import;
mod include;
mod inject;
mod line;

pub use env_condition::{ElseEnvRule, EndIfEnvRule, EnvCondition, IfEnvRule};
pub use envar::{Envar, EnvarExpressionRule, EnvarStatementRule};
pub use file::{File, FileRule};
pub use import::{Import, ImportRule};
pub use include::{Include, IncludeRule};
pub use inject::{Inject, InjectExpressionRule, InjectStatementRule};
pub use line::{Line, LineRule};

use crate::ast::TextSpan;
use crate::parser::Expect;
use crate::position::Span;
use crate::{NodeId, ParseResult, Parser};

fn parse_path(parser: &mut Parser<'_>) -> ParseResult<TextSpan> {
    if let Some(string) = parser.token().as_string() {
        let path = string.text();

        parser.advance()?;

        return Ok(path);
    }

    let start = parser.token().span().start;
    let mut end = start;

    while !(parser.token().is_eol() || parser.token().is_comment() || parser.token().is_eof()) {
        end = parser.token().span().end;
        parser.advance()?;
    }

    parser.advance()?;

    Ok(TextSpan::new(Span::new(start, end)))
}

fn emit_directive(parser: &mut Parser<'_>, item: NodeId) -> ParseResult<()> {
    let depth = parser.stream.blocks().depth();

    parser.emit_statement(depth, item, true);
    parser.consume(Expect::Eol)?;

    Ok(())
}
