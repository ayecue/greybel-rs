use crate::lexer::{AnyToken, Keyword, Punct};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Keyword(Keyword),
    Punct(Punct),
}

impl Trigger {
    pub fn of(token: &AnyToken) -> Option<Trigger> {
        token
            .punct()
            .map(Trigger::Punct)
            .or_else(|| token.keyword().map(Trigger::Keyword))
    }
}
