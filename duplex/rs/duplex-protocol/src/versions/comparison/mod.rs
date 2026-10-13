mod protocol;
mod types;

use std::collections::{HashMap, HashSet};

use super::AdditionTable;
use crate::error::DefinitionError;
use crate::model::{Definition, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Mode {
    Append,
    Exact,
}

pub(crate) struct Comparison<'a> {
    older: &'a Definition,
    old_additions: &'a AdditionTable,
    newer: &'a Definition,
    minor: u16,
    pairs: HashMap<usize, usize>,
    compared: HashSet<(usize, Mode)>,
    additions: AdditionTable,
}

impl<'a> Comparison<'a> {
    pub(crate) fn additions(
        older: &'a Definition,
        old_additions: &'a AdditionTable,
        newer: &'a Definition,
        minor: u16,
    ) -> Result<AdditionTable, DefinitionError> {
        let mut comparison = Comparison {
            older,
            old_additions,
            newer,
            minor,
            pairs: HashMap::new(),
            compared: HashSet::new(),
            additions: AdditionTable::default(),
        };

        comparison.compare()?;

        Ok(comparison.additions)
    }

    fn reject_addition(span: Span, name: &str) -> DefinitionError {
        DefinitionError::new(
            span,
            format!(
                "adds `{name}` to what a handshake carries, which peers read before they agree on a minor version"
            ),
        )
    }

    fn old_index(&self, name: &str) -> Option<usize> {
        self.older
            .types
            .iter()
            .position(|declaration| declaration.name == name)
    }

    fn new_index(&self, name: &str) -> Option<usize> {
        self.newer
            .types
            .iter()
            .position(|declaration| declaration.name == name)
    }
}
