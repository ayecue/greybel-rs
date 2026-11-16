mod doc;
mod ignore;
mod type_expr;

use std::sync::Arc;

pub use doc::{
    AliasDoc, DocComment, FunctionDoc, InvalidTag, ParamDoc, PropertyDoc, TemplateDoc,
    TextPosition, TypeDoc,
};
use greybel_core::{Ast, Comment, Span};
pub(crate) use ignore::IgnoreTable;
use rustc_hash::FxHashMap;
pub use type_expr::{FunctionTypeParameter, TypeExpr, TypeLiteral};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommentBlock {
    pub spans: Vec<Span>,
    pub text_spans: Vec<Span>,
    pub doc: Arc<DocComment>,
}

impl CommentBlock {
    /// The `@param` tags of the block with the source span of each name.
    pub fn param_names(&self) -> impl Iterator<Item = (&ParamDoc, Span)> {
        self.doc.params.iter().map(|param| {
            let position = param.name_position;
            let line = self.text_spans[position.line].start;
            let offset = |at: usize| line + u32::try_from(at).expect("comment offset fits u32");

            (
                param,
                Span::new(offset(position.start), offset(position.end)),
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CommentTable {
    pub blocks: Vec<CommentBlock>,
    block_index: FxHashMap<u32, usize>,
    pub ignores: IgnoreTable,
}

impl CommentTable {
    pub fn new(ast: &Ast) -> Self {
        let lines = ast.lines();
        let code_lines = Self::code_lines(ast);
        let own_line: Vec<(u32, Comment)> = ast
            .comments
            .iter()
            .filter(|comment| !comment.is_multiline)
            .map(|&comment| (lines.line(comment.span.start), comment))
            .filter(|(line, _)| code_lines.binary_search(line).is_err())
            .collect();
        let source = ast.source();
        let is_blank = |line: u32| {
            let start = lines.line_start(line).unwrap_or_default() as usize;
            let end = lines
                .line_start(line + 1)
                .map_or(source.len(), |end| end as usize);

            source[start..end].trim().is_empty()
        };
        let mut comments = Self {
            ignores: IgnoreTable::new(ast, &code_lines),
            ..Self::default()
        };
        let mut index = 0;

        while index < own_line.len() {
            let first = index;
            let mut last = own_line[index].0;

            index += 1;
            while index < own_line.len() && own_line[index].0 == last + 1 {
                last = own_line[index].0;
                index += 1;
            }

            let block = &own_line[first..index];
            let texts: Vec<&str> = block
                .iter()
                .map(|(_, comment)| ast.slice(comment.text_span))
                .collect();

            let next_code_line = code_lines
                .get(code_lines.partition_point(|&line| line <= last))
                .copied();
            let documented_line = next_code_line
                .filter(|&line| line == last + 1 || (line == last + 2 && is_blank(last + 1)));

            if let Some(documented_line) = documented_line {
                comments
                    .block_index
                    .insert(documented_line, comments.blocks.len());
            }

            comments.blocks.push(CommentBlock {
                spans: block.iter().map(|(_, comment)| comment.span).collect(),
                text_spans: block.iter().map(|(_, comment)| comment.text_span).collect(),
                doc: Arc::new(DocComment::parse(&texts)),
            });
        }

        comments
    }

    pub fn doc_at(&self, line: u32) -> Option<&Arc<DocComment>> {
        self.block_index
            .get(&line)
            .map(|&block| &self.blocks[block].doc)
    }

    fn code_lines(ast: &Ast) -> Vec<u32> {
        let lines = ast.lines();
        let bytes = ast.source().as_bytes();
        let mut comment_spans: Vec<Span> =
            ast.comments.iter().map(|comment| comment.span).collect();

        comment_spans.sort_unstable_by_key(|span| span.start);

        let mut comments = comment_spans.iter().peekable();
        let mut result = Vec::new();

        for line in 1..=lines.line_count() {
            let start = lines.line_start(line).unwrap_or_default() as usize;
            let end = lines
                .line_start(line + 1)
                .map_or(bytes.len(), |end| end as usize);
            let mut offset = start;

            while offset < end {
                while comments
                    .peek()
                    .is_some_and(|comment| comment.end as usize <= offset)
                {
                    comments.next();
                }

                if let Some(comment) = comments.peek()
                    && comment.start as usize <= offset
                {
                    offset = comment.end as usize;

                    continue;
                }

                if !bytes[offset].is_ascii_whitespace() {
                    result.push(line);

                    break;
                }

                offset += 1;
            }
        }

        result
    }
}
