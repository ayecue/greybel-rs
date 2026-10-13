use greybel_symbols::literals::FALSE;

use crate::diagnostics::BlockDiagnostic;
use crate::parser::{ChunkData, KeywordStatement, PendingBlock};
use crate::{Keyword, NodeId, NodeStore, ParseResult, Parser, Span};

pub struct EnvCondition {
    is_truthy: bool,
    start: Span,
    in_else: bool,
    if_body: Vec<NodeId>,
    else_body: Vec<NodeId>,
    if_chunk_data: Option<ChunkData>,
    else_chunk_data: Option<ChunkData>,
}

impl EnvCondition {
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        self.is_truthy
    }

    fn active_chunk_data(&mut self) -> &mut Option<ChunkData> {
        if self.in_else {
            &mut self.else_chunk_data
        } else {
            &mut self.if_chunk_data
        }
    }
}

impl PendingBlock for EnvCondition {
    fn body_mut(&mut self) -> &mut Vec<NodeId> {
        if self.in_else {
            &mut self.else_body
        } else {
            &mut self.if_body
        }
    }

    fn name(&self) -> &'static str {
        Keyword::IfEnv.as_str()
    }

    fn start(&self, _nodes: &NodeStore) -> u32 {
        self.start.start
    }

    fn complete(self: Box<Self>, parser: &mut Parser<'_>, _end: &Span) {
        parser.stream.chunk_data_mut().pop();
    }
}

const FALSE_VALUES: [&str; 2] = ["0", FALSE];

fn is_truthy(value: Option<&String>) -> bool {
    value.is_some_and(|v| !v.is_empty() && !FALSE_VALUES.contains(&v.as_str()))
}

fn raise_unmatched_end(parser: &mut Parser<'_>, directive: Keyword) -> ParseResult<()> {
    let span = parser.prev().span();
    let diagnostic = BlockDiagnostic::UnmatchedEnd {
        end: directive.as_str().to_owned(),
        block: Keyword::IfEnv.as_str(),
    };

    parser.raise_at(diagnostic, span)
}

#[derive(Debug, Clone, Copy)]
pub struct IfEnvRule;

impl KeywordStatement for IfEnvRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::IfEnv]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        let start = parser.token().span();

        parser.advance()?;

        let key = parser.token().value(parser.source()).into_owned();

        parser.advance()?;

        let condition = EnvCondition {
            is_truthy: is_truthy(parser.options().environment.get(&key)),
            start,
            in_else: false,
            if_body: Vec::new(),
            else_body: Vec::new(),
            if_chunk_data: None,
            else_chunk_data: None,
        };

        parser.stream.blocks_mut().push(condition);
        parser.stream.chunk_data_mut().push(ChunkData::default());

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElseEnvRule;

impl KeywordStatement for ElseEnvRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::ElseEnv]
    }

    fn parse(&self, parser: &mut Parser<'_>, keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;
        if !parser.stream.blocks().top_is::<EnvCondition>() {
            return raise_unmatched_end(parser, keyword);
        }

        let state = parser.stream.chunk_data_mut().pop();
        let condition = parser
            .stream
            .blocks_mut()
            .top_mut_as::<EnvCondition>()
            .expect("checked above");

        *condition.active_chunk_data() = state;
        condition.in_else = true;

        let next_state = condition.active_chunk_data().take().unwrap_or_default();

        parser.stream.chunk_data_mut().push(next_state);

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndIfEnvRule;

impl KeywordStatement for EndIfEnvRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::EndIfEnv]
    }

    fn parse(&self, parser: &mut Parser<'_>, keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let Some(mut condition) = parser.stream.blocks_mut().pop_as::<EnvCondition>() else {
            return raise_unmatched_end(parser, keyword);
        };

        *condition.active_chunk_data() = parser.stream.chunk_data_mut().pop();

        let (body, state) = if condition.is_truthy {
            (condition.if_body, condition.if_chunk_data)
        } else {
            (condition.else_body, condition.else_chunk_data)
        };

        parser.stream.blocks_mut().top_mut().body_mut().extend(body);
        if let Some(state) = state {
            parser.stream.chunk_data_mut().current_mut().merge(state);
        }

        Ok(())
    }
}
