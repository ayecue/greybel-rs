use crate::lexer::{AnyToken, Eof};
use crate::position::Span;

#[derive(Debug, Clone)]
pub struct Lookahead {
    current: AnyToken,
    previous: AnyToken,
    slot_count: u8,
}

pub type LookaheadSnapshot = Lookahead;

const PREVIOUS_SLOT_COUNT: u8 = 2;

fn before_input() -> AnyToken {
    Eof::new(Span::default()).into()
}

impl Default for Lookahead {
    fn default() -> Self {
        Self {
            current: before_input(),
            previous: before_input(),
            slot_count: 0,
        }
    }
}

impl Lookahead {
    #[must_use]
    pub fn current(&self) -> &AnyToken {
        &self.current
    }

    #[must_use]
    pub fn previous(&self) -> &AnyToken {
        &self.previous
    }

    #[must_use]
    pub fn has_previous(&self) -> bool {
        self.slot_count == PREVIOUS_SLOT_COUNT
    }

    pub fn push(&mut self, token: AnyToken) {
        self.previous = std::mem::replace(&mut self.current, token);
        self.slot_count = (self.slot_count + 1).min(PREVIOUS_SLOT_COUNT);
    }

    #[must_use]
    pub fn snapshot(&self) -> LookaheadSnapshot {
        self.clone()
    }

    pub fn restore(&mut self, snapshot: LookaheadSnapshot) {
        *self = snapshot;
    }
}
