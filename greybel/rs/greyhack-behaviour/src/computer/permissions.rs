//! Parsing and evaluation of Grey Hack's ten-character permission strings.

use std::fmt;

use crate::base::refusal::{Refusal, RuleResult};
use crate::computer::credentials::MAX_CREDENTIAL;
use crate::{ROOT_USER, path};

const FOLDER: u8 = b'd';
const NONE: u8 = b'-';
const LEN: usize = 10;
const TRIPLE: usize = 3;
const MAX_CHANGE: usize = 3;

/// Access-bit offset within an `rwx` triple; offset 0 is the folder flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read = 1,
    Write = 2,
    Execute = 3,
}

impl Permission {
    const fn letter(self) -> u8 {
        match self {
            Self::Read => b'r',
            Self::Write => b'w',
            Self::Execute => b'x',
        }
    }

    /// Parses the `has_permission` argument: `r`, `w`, or `x` alone (`FileIntrinsics.has_permission`).
    #[must_use]
    pub fn from_mode(mode: &str) -> Option<Self> {
        let mut letters = mode.chars();

        match (letters.next(), letters.next()) {
            (Some(letter), None) => Self::from_letter(letter),
            _ => None,
        }
    }

    fn from_letter(letter: char) -> Option<Self> {
        Some(match letter {
            'r' => Self::Read,
            'w' => Self::Write,
            'x' => Self::Execute,
            _ => return None,
        })
    }
}

/// Account attributes used for permission checks, independent of world user records.
#[derive(Debug, Clone, Copy)]
pub struct Principal<'a> {
    pub name: &'a str,
    pub groups: &'a [String],
}

impl<'a> Principal<'a> {
    #[must_use]
    pub const fn new(name: &'a str, groups: &'a [String]) -> Self {
        Self { name, groups }
    }

    #[must_use]
    pub fn is_root(self) -> bool {
        self.name == ROOT_USER
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PermissionSet([u8; LEN]);

impl PermissionSet {
    /// `drwxrwx---`: only the owner and the group use the folder.
    pub const GROUP_FOLDER: Self = Self(*b"drwxrwx---");
    /// `-rw-r-----`: the owner reads and writes the file and the group reads it.
    pub const GROUP_READ_FILE: Self = Self(*b"-rw-r-----");
    /// `drwxrwxr-x`: the owner and group use the folder; others read and enter it.
    pub const HOME_FOLDER: Self = Self(*b"drwxrwxr-x");
    /// `-rwxrwxrwx`: every user reads, writes, and runs the file.
    pub const OPEN_FILE: Self = Self(*b"-rwxrwxrwx");
    /// `drwxrwxrwx`: every user reads, writes, and enters the folder.
    pub const OPEN_FOLDER: Self = Self(*b"drwxrwxrwx");
    /// `drwx------`: only the owner uses the folder.
    pub const PRIVATE_FOLDER: Self = Self(*b"drwx------");

    /// Returns `drwxr-xr-x` when `folder` is true and `-rwxr-xr-x` otherwise.
    #[must_use]
    pub const fn default_for(folder: bool) -> Self {
        let mut bits = *b"-rwxr-xr-x";

        if folder {
            bits[0] = FOLDER;
        }

        Self(bits)
    }

    /// Accepts exactly ten ASCII characters from `d-rwx`; otherwise returns `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bytes: [u8; LEN] = text.as_bytes().try_into().ok()?;

        bytes
            .iter()
            .all(|byte| b"d-rwx".contains(byte))
            .then_some(Self(bytes))
    }

    #[must_use]
    pub fn is_folder(&self) -> bool {
        self.0[0] == FOLDER
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("ASCII")
    }

    /// Checks root access first, then owner, group, or other bits. Access is denied when the principal
    /// is missing.
    #[must_use]
    pub fn allows(
        &self,
        access: Permission,
        owner: &str,
        group: &str,
        principal: Option<Principal<'_>>,
    ) -> bool {
        let Some(principal) = principal else {
            return false;
        };

        if principal.is_root() {
            // `Permisos.TienePermiso`: root cannot execute a regular file when both the owner and
            // group execute bits are unset.
            return access != Permission::Execute
                || self.is_folder()
                || self.0[Permission::Execute as usize] == b'x'
                || self.0[Permission::Execute as usize + TRIPLE] == b'x'
                || self.0[Permission::Execute as usize + TRIPLE * 2] == b'x';
        }

        let triple = if principal.name == owner {
            0
        } else if principal.groups.iter().any(|candidate| candidate == group) {
            1
        } else {
            2
        };

        self.0[access as usize + triple * TRIPLE] == access.letter()
    }

    /// Applies `+` or `-` with up to three `rwx` letters to the `u`, `g`, or `o` triple. Malformed
    /// changes return `None`.
    #[must_use]
    pub fn with_change(&self, change: &str) -> Option<Self> {
        let sign = change.find(['+', '-'])?;
        let mut chars = change.chars();
        let triple = match chars.next()? {
            'u' => 0,
            'g' => 1,
            'o' => 2,
            _ => return None,
        };

        if !change.chars().skip(2).all(|c| matches!(c, 'r' | 'w' | 'x')) {
            return None;
        }

        let letters = &change[sign + 1..];

        if letters.chars().count() > MAX_CHANGE {
            return None;
        }

        let grant = change.as_bytes()[sign] == b'+';
        let mut bits = self.0;

        for letter in letters.chars() {
            let access = Permission::from_letter(letter)?;
            let at = access as usize + triple * TRIPLE;

            bits[at] = if grant { access.letter() } else { NONE };
        }

        Some(Self(bits))
    }
}

impl fmt::Display for PermissionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for PermissionSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PermissionSet({self})")
    }
}

/// File attribute changed by `set_owner` or `set_group`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOwnership {
    Owner,
    Group,
}

impl FileOwnership {
    /// Validates a new owner or group name (`ValidateOwnerValue` / `ValidateGroupValue`).
    pub fn check(self, name: &str) -> RuleResult {
        if !path::is_alphanumeric(name) {
            Err(Refusal::FileOwnershipInvalidName(self))
        } else if name.chars().count() > MAX_CREDENTIAL {
            Err(Refusal::FileOwnershipNameTooLong(self))
        } else {
            Ok(())
        }
    }
}
