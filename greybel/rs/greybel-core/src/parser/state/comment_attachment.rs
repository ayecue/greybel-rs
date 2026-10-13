use crate::ast::NodeId;
use crate::position::Span;

#[derive(Debug, Default)]
pub struct CommentAttachment {
    lead_comments: Vec<Span>,
    last_statement: Option<NodeId>,
}

impl CommentAttachment {
    pub fn push_comment(&mut self, comment: Span) {
        self.lead_comments.push(comment);
    }

    #[must_use]
    pub fn has_lead_comments(&self) -> bool {
        !self.lead_comments.is_empty()
    }

    pub fn take_comments(&mut self) -> Vec<Span> {
        std::mem::take(&mut self.lead_comments)
    }

    #[must_use]
    pub fn last_statement(&self) -> Option<NodeId> {
        self.last_statement
    }

    pub fn set_last_statement(&mut self, statement: Option<NodeId>) {
        self.last_statement = statement;
    }
}
