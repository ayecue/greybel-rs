mod chars;
mod dispatch;
pub mod greybel;
pub mod greyscript;
mod lexeme;
mod scan;
pub mod state;
mod stream;
pub mod token;
mod tokens;

use std::sync::{Arc, OnceLock};

pub(crate) use chars::{CARRIAGE_RETURN, LINE_FEED, TAB, identifier_end};
pub use chars::{
    is_end_of_line, is_identifier_part, is_identifier_start, is_whitespace, is_windows_newline,
};
use dispatch::{TokenStart, TokenStartTable};
use lexeme::{CompoundKeywordTable, CompoundStart, LexemeTable};
pub use stream::{TokenSnapshot, TokenStream};
pub use token::{
    AnyToken, BooleanLiteral, Comment, Eof, Eol, Identifier, Invalid, KEYWORDS, Keyword,
    KeywordToken, NilLiteral, NumericLiteral, PUNCTUATORS, PendingToken, Punct, Punctuator,
    SliceOperator, StringLiteral, Token, TokenKind, keyword, punct,
};
pub use tokens::TokenIter;

use crate::dialect::{Dialect, Grammar, LexerRuleTable, Version};
use crate::error::ParseResult;
use crate::position::LineIndex;

pub type LexResult = ParseResult<AnyToken>;

pub(crate) fn offset(index: usize) -> u32 {
    u32::try_from(index).expect("sources are smaller than 4 GiB")
}

#[derive(Debug, Clone)]
pub struct Lexer<'src> {
    source: &'src str,
    lines: Arc<OnceLock<LineIndex>>,
    grammar: &'static Grammar,
}

#[derive(Debug)]
pub(crate) struct Lexicon {
    keywords: LexemeTable<Keyword>,
    compounds: CompoundKeywordTable,
    punctuators: LexemeTable<Punct>,
    starts: TokenStartTable,
}

impl Lexicon {
    pub(crate) fn new(
        rules: LexerRuleTable,
        keywords: &[&[Keyword]],
        punctuators: &[&[Punct]],
    ) -> Self {
        let compounds = CompoundKeywordTable::new(keywords);
        let first_words = compounds.first_words();
        let mut keyword_tables: Vec<&[Keyword]> = keywords.to_vec();

        keyword_tables.push(&first_words);

        Self {
            keywords: LexemeTable::new(&keyword_tables),
            compounds,
            punctuators: LexemeTable::new(punctuators),
            starts: TokenStartTable::new(
                rules,
                punctuators.iter().flat_map(|table| table.iter()).copied(),
            ),
        }
    }
}

impl<'src> Lexer<'src> {
    #[must_use]
    pub fn new(source: &'src str, dialect: Dialect, version: Version) -> Self {
        Self::with_grammar(source, Grammar::get(dialect, version, false))
    }

    #[must_use]
    pub fn with_grammar(source: &'src str, grammar: &'static Grammar) -> Self {
        Self {
            source,
            lines: Arc::default(),
            grammar,
        }
    }

    #[must_use]
    pub fn source(&self) -> &'src str {
        self.source
    }

    #[must_use]
    pub fn grammar(&self) -> &'static Grammar {
        self.grammar
    }

    #[must_use]
    pub fn lines(&self) -> &LineIndex {
        self.lines.get_or_init(|| LineIndex::new(self.source))
    }

    pub(crate) fn shared_lines(&self) -> Arc<OnceLock<LineIndex>> {
        Arc::clone(&self.lines)
    }

    #[must_use]
    pub fn has_line_break(&self, from: u32, to: u32) -> bool {
        let (from, to) = (from as usize, to as usize);

        self.source.as_bytes()[from.min(to)..to.max(from)]
            .iter()
            .any(|&byte| is_end_of_line(byte))
    }

    #[must_use]
    pub fn keyword(&self, word: &str) -> Option<Keyword> {
        self.grammar.lexicon.keywords.get(word)
    }

    pub(crate) fn compound_start(&self, keyword: Keyword) -> Option<&'static CompoundStart> {
        self.grammar.lexicon.compounds.start(keyword)
    }

    #[must_use]
    pub fn punct_at(&self, input: &[u8]) -> Option<Punct> {
        self.grammar.lexicon.punctuators.longest_prefix(input)
    }

    pub(crate) fn token_start(&self, byte: u8) -> TokenStart {
        self.grammar.lexicon.starts.get(byte)
    }

    #[must_use]
    pub fn tokens(&self) -> TokenIter<'_, 'src> {
        TokenIter::new(self, TokenStream::new())
    }

    #[must_use]
    pub fn tokens_unsafe(&self) -> TokenIter<'_, 'src> {
        TokenIter::new(self, TokenStream::new_unsafe())
    }
}
