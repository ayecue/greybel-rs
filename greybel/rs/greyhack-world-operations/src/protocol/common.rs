//! Conversions shared by every domain: outcomes, refusals, names, and credentials.

use super::{Credential, Failure, FileOwnership, NameKind, Outcome, Refusal};

impl From<bool> for Outcome {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

/// Converts a change result to `true` on success or the failure reason.
impl From<Result<(), Failure>> for Outcome {
    fn from(result: Result<(), Failure>) -> Self {
        result.map_or_else(Self::Failure, |()| Self::True)
    }
}

impl From<greyhack_behaviour::NameKind> for NameKind {
    fn from(kind: greyhack_behaviour::NameKind) -> Self {
        match kind {
            greyhack_behaviour::NameKind::File => Self::File,
            greyhack_behaviour::NameKind::Folder => Self::Folder,
            greyhack_behaviour::NameKind::New => Self::New,
        }
    }
}

impl From<greyhack_behaviour::Credential> for Credential {
    fn from(credential: greyhack_behaviour::Credential) -> Self {
        match credential {
            greyhack_behaviour::Credential::UserName => Self::UserName,
            greyhack_behaviour::Credential::Password => Self::Password,
            greyhack_behaviour::Credential::GroupName => Self::GroupName,
        }
    }
}

impl From<greyhack_behaviour::FileOwnership> for FileOwnership {
    fn from(owner: greyhack_behaviour::FileOwnership) -> Self {
        match owner {
            greyhack_behaviour::FileOwnership::Owner => Self::Owner,
            greyhack_behaviour::FileOwnership::Group => Self::Group,
        }
    }
}

impl From<greyhack_behaviour::Refusal> for Refusal {
    fn from(refusal: greyhack_behaviour::Refusal) -> Self {
        use greyhack_behaviour::Refusal as Rule;

        match refusal {
            Rule::InvalidArguments => Self::InvalidArguments,
            Rule::TooLong(credential) => Self::TooLong(credential.into()),
            Rule::NameTooLong => Self::NameTooLong,
            Rule::FileOwnershipInvalidName(owner) => Self::FileOwnershipInvalidName(owner.into()),
            Rule::FileOwnershipNameTooLong(owner) => Self::FileOwnershipNameTooLong(owner.into()),
            Rule::InvalidName(kind) => Self::InvalidName(kind.into()),
            Rule::LongName => Self::LongName,
            Rule::ContentTooLong => Self::ContentTooLong,
            Rule::InvalidProcessName => Self::InvalidProcessName,
            Rule::ProcessNameTooLong => Self::ProcessNameTooLong,
            Rule::ReservedProcessName(name) => Self::ReservedProcessName(name),
            Rule::EmptyOrder => Self::EmptyOrder,
            Rule::TooManyOrderCoins => Self::TooManyOrderCoins,
            Rule::OrderPriceTooHigh => Self::OrderPriceTooHigh,
            Rule::InvalidMailAddress => Self::InvalidMailAddress,
            Rule::MailSubjectTooLong => Self::MailSubjectTooLong,
            Rule::MailMessageTooLong => Self::MailMessageTooLong,
            Rule::InvalidPassword => Self::InvalidPassword,
            Rule::PasswordTooLong => Self::PasswordTooLong,
            Rule::InvalidUserCredentials => Self::InvalidUserCredentials,
            Rule::InvalidGroupCredentials => Self::InvalidGroupCredentials,
            Rule::InvalidWalletPassword => Self::InvalidWalletPassword,
            Rule::WalletPasswordTooLong => Self::WalletPasswordTooLong,
            Rule::InvalidWalletCredentials => Self::InvalidWalletCredentials,
            Rule::WalletCredentialsTooLong => Self::WalletCredentialsTooLong,
            Rule::InvalidSubwalletCredentials => Self::InvalidSubwalletCredentials,
            Rule::SubwalletCredentialsTooLong => Self::SubwalletCredentialsTooLong,
            Rule::CtfPasswordTooLong => Self::CtfPasswordTooLong,
            Rule::InvalidCtfPassword => Self::InvalidCtfPassword,
        }
    }
}
