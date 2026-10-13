#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeCommentContainer {
    pub lead_comments: Vec<Span>,
    pub trailing_comments: Vec<Span>,
    pub end_trailing_comments: Vec<Span>,
}

use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comment {
    pub span: Span,
    pub text_span: Span,
    pub is_multiline: bool,
}
