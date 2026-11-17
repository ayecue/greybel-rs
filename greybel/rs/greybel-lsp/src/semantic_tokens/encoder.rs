use gen_lsp_types as lsp;
use greybel_core::highlight::{HighlightKind, HighlightToken};
use greybel_core::{Ast, ColumnSettings};

pub(super) struct TokenEncoder<'a> {
    ast: &'a Ast,
    tokens: Vec<lsp::SemanticToken>,
    line: u32,
    character: u32,
}

impl<'a> TokenEncoder<'a> {
    pub(super) fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            tokens: Vec::new(),
            line: 0,
            character: 0,
        }
    }

    pub(super) fn into_tokens(self) -> Vec<lsp::SemanticToken> {
        self.tokens
    }

    pub(super) fn push(&mut self, token: HighlightToken) {
        let HighlightToken { span, kind } = token;
        let (start, end) = (span.start, span.end);
        let text = &self.ast.source()[start as usize..end as usize];
        let mut offset = start;

        for line in text.split('\n') {
            let segment = line.strip_suffix('\r').unwrap_or(line);

            if !segment.is_empty() {
                self.push_segment(
                    offset,
                    offset
                        + u32::try_from(segment.len()).expect("value fits the target integer type"),
                    kind,
                );
            }

            offset += u32::try_from(line.len()).expect("value fits the target integer type") + 1;
        }
    }

    fn push_segment(&mut self, start: u32, end: u32, kind: HighlightKind) {
        let lines = self.ast.lines();
        let from = lines.position(start, ColumnSettings::default());
        let to = lines.position(end, ColumnSettings::default());
        let line = from.line - 1;
        let character = from.character - 1;
        let delta_line = line - self.line;
        let delta_start = if delta_line == 0 {
            character - self.character
        } else {
            character
        };

        self.tokens.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length: to.character - from.character,
            token_type: kind as u32,
            token_modifiers_bitset: 0,
        });
        self.line = line;
        self.character = character;
    }
}
