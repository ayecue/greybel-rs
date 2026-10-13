mod doc;
mod type_expr;

pub use doc::{DocComment, FunctionDoc, InvalidTag, ParamDoc, PropertyDoc, TemplateDoc, TypeDoc};
use greybel_core::{Ast, Span};
use rustc_hash::FxHashMap;
pub use type_expr::TypeExpr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommentBlock {
    pub spans: Vec<Span>,
    pub doc: DocComment,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CommentTable {
    pub blocks: Vec<CommentBlock>,
    block_index: FxHashMap<u32, usize>,
}

impl CommentTable {
    pub fn new(ast: &Ast) -> Self {
        let source = ast.source();
        let lines = ast.lines();
        let line_text = |line: u32| -> &str {
            let start = lines.line_start(line).unwrap_or_default() as usize;
            let end = lines
                .line_start(line + 1)
                .map_or(source.len(), |end| end as usize);

            &source[start..end]
        };
        let mut own_line: Vec<(u32, Span, &str)> = Vec::new();

        for comment in &ast.comments {
            if comment.is_multiline {
                continue;
            }

            let line = lines.line(comment.span.start);
            let start = lines.line_start(line).unwrap_or_default();
            let before = &source[start as usize..comment.span.start as usize];

            if before.trim().is_empty() {
                own_line.push((line, comment.span, ast.slice(comment.text_span)));
            }
        }

        let mut comments = Self::default();
        let mut index = 0;

        while index < own_line.len() {
            let (mut last, span, text) = own_line[index];
            let mut spans = vec![span];
            let mut texts = vec![text];

            index += 1;
            while index < own_line.len() && own_line[index].0 == last + 1 {
                (last, _, _) = own_line[index];
                spans.push(own_line[index].1);
                texts.push(own_line[index].2);
                index += 1;
            }

            let mut documented_line = last + 1;

            if documented_line <= lines.line_count() && line_text(documented_line).trim().is_empty()
            {
                documented_line += 1;
            }

            let documents = documented_line <= lines.line_count()
                && !line_text(documented_line).trim().is_empty()
                && own_line
                    .get(index)
                    .is_none_or(|&(line, ..)| line != documented_line);

            if documents {
                comments
                    .block_index
                    .insert(documented_line, comments.blocks.len());
            }

            comments.blocks.push(CommentBlock {
                spans,
                doc: DocComment::parse(&texts),
            });
        }

        comments
    }

    pub fn doc_at(&self, line: u32) -> Option<&DocComment> {
        self.block_index
            .get(&line)
            .map(|&block| &self.blocks[block].doc)
    }
}
