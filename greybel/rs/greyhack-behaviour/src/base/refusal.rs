//! Shared argument-refusal rules for runtimes and the game server.
//! Each refusal identifies the validation failure; the runtime formats it for the intrinsic.

use crate::computer::permissions::FileOwnership;

/// Argument validation failure. Intrinsics raise some failures as runtime errors
/// ([`Refusal::is_raised`]) and return the others as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Raised as `<intrinsic>: Invalid arguments`.
    InvalidArguments,
    /// Credential exceeds 15 characters; raised by the intrinsic.
    TooLong(Credential),
    /// New filename exceeds 128 characters; raised by the intrinsic.
    NameTooLong,
    /// `set_owner` or `set_group` name contains non-alphanumeric characters; raised by the intrinsic.
    FileOwnershipInvalidName(FileOwnership),
    /// `set_owner` or `set_group` name exceeds 15 characters; raised by the intrinsic.
    FileOwnershipNameTooLong(FileOwnership),
    /// Name contains characters outside `[a-zA-Z0-9_.-]`.
    InvalidName(NameKind),
    /// Name exceeds 128 characters.
    LongName,
    /// File content exceeds 160,000 UTF-16 code units.
    ContentTooLong,
    /// `rshell_client` process name contains characters outside `[a-zA-Z0-9_.-]`.
    InvalidProcessName,
    /// `rshell_client` process name exceeds 24 characters.
    ProcessNameTooLong,
    /// `rshell_client` process name is reserved by the game.
    ReservedProcessName(String),
    /// Market order amount or price is not greater than 0.
    EmptyOrder,
    /// Market order exceeds the per-trade coin limit.
    TooManyOrderCoins,
    /// Market order exceeds the per-trade unit-price limit.
    OrderPriceTooHigh,
    /// Mail address lacks a name before `@` or exceeds the length limit.
    InvalidMailAddress,
    MailSubjectTooLong,
    MailMessageTooLong,
    /// Password contains non-alphanumeric characters.
    InvalidPassword,
    /// Password exceeds 15 characters.
    PasswordTooLong,
    /// `create_user` user name or password contains non-alphanumeric characters.
    InvalidUserCredentials,
    /// `create_group` user or group name contains non-alphanumeric characters.
    InvalidGroupCredentials,
    /// New wallet or coin password contains non-alphanumeric characters.
    InvalidWalletPassword,
    /// New wallet or coin password exceeds 16 characters.
    WalletPasswordTooLong,
    /// `create_wallet` name or password contains non-alphanumeric characters.
    InvalidWalletCredentials,
    /// `create_wallet` name or password exceeds 16 characters.
    WalletCredentialsTooLong,
    /// `create_subwallet` user name or password contains characters outside `[a-zA-Z0-9_.-]`.
    InvalidSubwalletCredentials,
    /// `create_subwallet` user name or password exceeds 15 characters.
    SubwalletCredentialsTooLong,
    /// New CTF account password exceeds 32 characters.
    CtfPasswordTooLong,
    /// New CTF account password contains characters other than ASCII letters and digits.
    InvalidCtfPassword,
}

impl Refusal {
    /// Checks whether the intrinsic raises this refusal instead of returning its text. Programs apply
    /// these checks before sending an operation.
    #[must_use]
    pub const fn is_raised(&self) -> bool {
        matches!(
            self,
            Self::InvalidArguments
                | Self::TooLong(_)
                | Self::NameTooLong
                | Self::FileOwnershipInvalidName(_)
                | Self::FileOwnershipNameTooLong(_)
        )
    }
}

/// Credential kind rejected for excessive length by `create_user` or `create_group`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credential {
    UserName,
    Password,
    GroupName,
}

/// Kind of name that failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    /// New file passed to `touch`.
    File,
    /// New folder passed to `create_folder`.
    Folder,
    /// Destination name used by `rename`, `copy`, `move`, or `symlink`.
    New,
}

/// Rule result containing `Err` when argument validation fails.
pub type RuleResult = Result<(), Refusal>;
