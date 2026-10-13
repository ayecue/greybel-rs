use crate::parser::{Precedence, PrecedenceTable};

/// MiniScript revision used by a dialect. `V1_6` pins upstream commit `0d7a2d3`; `Latest` pins `b451031`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Version {
    V1_6,
    #[default]
    Latest,
}

impl Version {
    pub const ALL: [Self; 2] = [Self::V1_6, Self::Latest];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::V1_6 => "1.6",
            Self::Latest => "latest",
        }
    }

    /// Returns operator precedence for this version. `@` binds below `^` through 1.6.1 and above it from 1.6.2.
    #[must_use]
    pub fn precedence(self) -> PrecedenceTable {
        let table = PrecedenceTable::new(&[
            Precedence::Or,
            Precedence::And,
            Precedence::Not,
            Precedence::Isa,
            Precedence::Comparison,
            Precedence::AddSub,
            Precedence::MulDiv,
            Precedence::UnaryMinus,
            Precedence::New,
            Precedence::AddressOf,
            Precedence::Power,
            Precedence::Call,
        ]);

        match self {
            Self::V1_6 => table,
            Self::Latest => table.swap(Precedence::Power, Precedence::AddressOf),
        }
    }

    /// Reports whether `else if` can continue a single-line `if`, as in MiniScript 1.6.2.
    #[must_use]
    pub const fn single_line_else_if(self) -> bool {
        matches!(self, Self::Latest)
    }

    /// Reports whether a value-less `return` can precede `else` in a single-line `if`, as in MiniScript 1.6.1.
    #[must_use]
    pub const fn return_before_else(self) -> bool {
        matches!(self, Self::Latest)
    }

    /// Reports whether an out-of-loop `break` is diagnosed as an unmatched block opener. MiniScript 1.6 detects
    /// it only at end of code; later versions identify the `break`.
    #[must_use]
    pub const fn break_outside_loop_is_unmatched_block(self) -> bool {
        matches!(self, Self::V1_6)
    }

    /// Reports whether single-line `if` errors identify the underlying construct (a loop, or `=` for `==`),
    /// matching MiniScript 1.6.2; version 1.6 identifies the `if`.
    #[must_use]
    pub const fn explains_single_line_if_errors(self) -> bool {
        matches!(self, Self::Latest)
    }

    /// Reports whether compound assignments such as `+=` are supported, starting in MiniScript 1.6.2.
    #[must_use]
    pub const fn compound_assignments(self) -> bool {
        matches!(self, Self::Latest)
    }

    /// Reports whether a line break ends an unclosed string literal.
    #[must_use]
    pub const fn strings_end_at_line_break(self) -> bool {
        matches!(self, Self::Latest)
    }
}
