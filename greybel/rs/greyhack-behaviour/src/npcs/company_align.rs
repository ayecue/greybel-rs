//! NPC stance towards the company when it knows of a zero-day (`NpcInfo.CompanyAlign`).

/// Company stance of an NPC who knows of a zero-day (`NpcInfo.CompanyAlign`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompanyAlign {
    /// The NPC takes no side.
    #[default]
    None = 0,
    Good = 1,
    Bad = 2,
}

impl CompanyAlign {
    /// Returns the alignment numbered `code` in Grey Hack order.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::Good),
            2 => Some(Self::Bad),
            _ => None,
        }
    }

    /// Returns Grey Hack's numeric alignment code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Returns the alignment name used by Grey Hack (`CompanyAlign.ToString`).
    #[must_use]
    pub const fn game_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Good => "GOOD",
            Self::Bad => "BAD",
        }
    }
}
