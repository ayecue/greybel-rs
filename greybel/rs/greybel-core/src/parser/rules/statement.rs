use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::Parser;

pub trait KeywordStatement: Sync {
    fn keywords(&self) -> &[Keyword];
    fn parse(&self, parser: &mut Parser<'_>, keyword: Keyword) -> ParseResult<()>;
}
