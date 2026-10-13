use std::collections::HashMap;

use regex_automata::dfa::Automaton;
use regex_automata::dfa::dense::{Builder, Config};
use regex_automata::util::primitives::StateID;
use regex_automata::util::{start, syntax};
use regex_automata::{Anchored, MatchKind};

const MAX_TABLE_BYTES: usize = 256 * 1024;
const BYTES: usize = 256;

#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error(
        "takes a table of more than 256 KiB: repeat a class with `+` or `*` and require a length instead"
    )]
    TooLarge,
    #[error("isn't a pattern a table can match: {0}")]
    Invalid(#[source] Box<regex_automata::dfa::dense::BuildError>),
    #[error("has no start: {0}")]
    NoStart(#[source] regex_automata::dfa::StartError),
    #[error("needs to look around a byte a table can't")]
    LookAround,
    #[error("takes more states than a table contains")]
    StateOverflow(#[source] std::num::TryFromIntError),
}

impl PatternError {
    #[must_use]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.to_string().starts_with(prefix)
    }
}

/// Regular expression represented by a deterministic byte-transition table.
///
/// For state `s` and byte class `c`, `base[s] + c` selects a candidate transition and `owner` validates it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternTable {
    /// Maps bytes to classes; bytes in a class share outgoing transitions from every state.
    pub classes: [u8; BYTES],
    /// Initial state. State 0 is the dead state.
    pub start: u16,
    /// Per-state base offsets into `next` and `owner`.
    pub base: Vec<u32>,
    /// Destination states for transition slots.
    pub next: Vec<u16>,
    /// Owning states for transition slots, used to reject `base + class` collisions.
    pub owner: Vec<u16>,
    /// End-of-input acceptance flags by state.
    pub accepting: Vec<bool>,
}

impl PatternTable {
    /// Dead state 0 for failed matches.
    pub const DEAD: u16 = 0;

    /// Compiles `pattern` for full-string matching.
    ///
    /// # Errors
    ///
    /// Returns an error if this table cannot represent the pattern or exceeds 256 KiB.
    pub fn compile(pattern: &str) -> Result<Self, PatternError> {
        let dfa = Builder::new()
            .syntax(syntax::Config::new().unicode(true).utf8(true))
            .configure(
                Config::new()
                    .match_kind(MatchKind::All)
                    .start_kind(regex_automata::dfa::StartKind::Anchored)
                    .minimize(true)
                    .byte_classes(true)
                    .dfa_size_limit(Some(MAX_TABLE_BYTES * 4))
                    .determinize_size_limit(Some(MAX_TABLE_BYTES * 4)),
            )
            .build(pattern)
            .map_err(|error| {
                if error.is_size_limit_exceeded() {
                    PatternError::TooLarge
                } else {
                    PatternError::Invalid(Box::new(error))
                }
            })?;
        let byte_classes = dfa.byte_classes();
        let alphabet = byte_classes.alphabet_len() - 1;
        let mut representatives = vec![0_u8; alphabet];
        let mut classes = [0_u8; BYTES];

        for byte in 0..=u8::MAX {
            let class = byte_classes.get(byte);

            classes[usize::from(byte)] = class;
            representatives[usize::from(class)] = byte;
        }

        let first = dfa
            .start_state(&start::Config::new().anchored(Anchored::Yes))
            .map_err(PatternError::NoStart)?;
        let mut ids: HashMap<StateID, u16> = HashMap::from([(first, 1)]);
        let mut order = vec![first];
        let mut rows: Vec<Vec<(usize, u16)>> = vec![Vec::new()];
        let mut accepting = vec![false];
        let mut next = 0;

        while let Some(&state) = order.get(next) {
            next += 1;
            accepting.push(dfa.is_match_state(dfa.next_eoi_state(state)));

            let mut row = Vec::new();

            for (class, &byte) in representatives.iter().enumerate() {
                let target = dfa.next_state(state, byte);

                if dfa.is_quit_state(target) {
                    return Err(PatternError::LookAround);
                }

                if dfa.is_dead_state(target) {
                    continue;
                }

                let id = if let Some(&id) = ids.get(&target) {
                    id
                } else {
                    let id = u16::try_from(order.len() + 1).map_err(PatternError::StateOverflow)?;

                    ids.insert(target, id);
                    order.push(target);

                    id
                };

                row.push((class, id));
            }

            rows.push(row);
        }

        let table = Self::comb(classes, &rows, accepting, alphabet);

        if table.bytes() > MAX_TABLE_BYTES {
            return Err(PatternError::TooLarge);
        }

        Ok(table)
    }

    #[must_use]
    pub fn matches(&self, bytes: &[u8]) -> bool {
        let mut state = self.start;

        for &byte in bytes {
            let slot = self.base[usize::from(state)] as usize
                + usize::from(self.classes[usize::from(byte)]);

            if self.owner[slot] != state {
                return false;
            }

            state = self.next[slot];
        }

        self.accepting[usize::from(state)]
    }

    fn comb(
        classes: [u8; BYTES],
        rows: &[Vec<(usize, u16)>],
        accepting: Vec<bool>,
        alphabet: usize,
    ) -> Self {
        let mut base = vec![0_u32; rows.len()];
        let mut next: Vec<u16> = Vec::new();
        let mut owner: Vec<u16> = Vec::new();

        for (state, row) in rows.iter().enumerate().skip(1) {
            let offset = (0..=owner.len())
                .find(|&offset| {
                    row.iter().all(|&(class, _)| {
                        owner
                            .get(offset + class)
                            .is_none_or(|&owner| owner == Self::DEAD)
                    })
                })
                .expect("the pattern finds an unused offset for each state's transitions");
            let state = u16::try_from(state).expect("the pattern's states fit 16 bits");

            next.resize(next.len().max(offset + alphabet), Self::DEAD);
            owner.resize(owner.len().max(offset + alphabet), Self::DEAD);
            for &(class, target) in row {
                next[offset + class] = target;
                owner[offset + class] = state;
            }

            base[usize::from(state)] =
                u32::try_from(offset).expect("the pattern's offsets fit 32 bits");
        }

        Self {
            classes,
            start: 1,
            base,
            next,
            owner,
            accepting,
        }
    }

    fn bytes(&self) -> usize {
        BYTES + self.base.len() * 4 + self.next.len() * 4 + self.accepting.len()
    }
}
