//! Conversions shared by every domain: outcomes, refusals, names, and credentials.

use super::{Credential, FileOwnership, IntoProtocol, NameKind, Outcome, Refusal, Side};

impl IntoProtocol for bool {
    type Output = Outcome;

    fn into_protocol(self) -> Self::Output {
        let value = self;

        if value { Outcome::True } else { Outcome::False }
    }
}

impl IntoProtocol for greyhack_behaviour::NameKind {
    type Output = NameKind;

    fn into_protocol(self) -> Self::Output {
        let kind = self;

        match kind {
            greyhack_behaviour::NameKind::File => NameKind::File,
            greyhack_behaviour::NameKind::Folder => NameKind::Folder,
            greyhack_behaviour::NameKind::New => NameKind::New,
        }
    }
}

impl IntoProtocol for greyhack_behaviour::Credential {
    type Output = Credential;

    fn into_protocol(self) -> Self::Output {
        let credential = self;

        match credential {
            greyhack_behaviour::Credential::UserName => Credential::UserName,
            greyhack_behaviour::Credential::Password => Credential::Password,
            greyhack_behaviour::Credential::GroupName => Credential::GroupName,
        }
    }
}

impl IntoProtocol for greyhack_behaviour::FileOwnership {
    type Output = FileOwnership;

    fn into_protocol(self) -> Self::Output {
        let owner = self;

        match owner {
            greyhack_behaviour::FileOwnership::Owner => FileOwnership::Owner,
            greyhack_behaviour::FileOwnership::Group => FileOwnership::Group,
        }
    }
}

impl IntoProtocol for greyhack_behaviour::Side {
    type Output = Side;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::Buy => Side::Buy,
            Self::Sell => Side::Sell,
        }
    }
}

impl IntoProtocol for greyhack_behaviour::Refusal {
    type Output = Refusal;

    fn into_protocol(self) -> Self::Output {
        let refusal = self;

        use greyhack_behaviour::Refusal as Rule;

        match refusal {
            Rule::InvalidArguments => Refusal::InvalidArguments,
            Rule::TooLong(credential) => Refusal::TooLong(credential.into_protocol()),
            Rule::NameTooLong => Refusal::NameTooLong,
            Rule::FileOwnershipInvalidName(owner) => {
                Refusal::FileOwnershipInvalidName(owner.into_protocol())
            }
            Rule::FileOwnershipNameTooLong(owner) => {
                Refusal::FileOwnershipNameTooLong(owner.into_protocol())
            }
            Rule::InvalidName(kind) => Refusal::InvalidName(kind.into_protocol()),
            Rule::LongName => Refusal::LongName,
            Rule::ContentTooLong => Refusal::ContentTooLong,
            Rule::InvalidProcessName => Refusal::InvalidProcessName,
            Rule::ProcessNameTooLong => Refusal::ProcessNameTooLong,
            Rule::ReservedProcessName(name) => Refusal::ReservedProcessName(name),
            Rule::EmptyOrder => Refusal::EmptyOrder,
            Rule::TooManyOrderCoins => Refusal::TooManyOrderCoins,
            Rule::OrderPriceTooHigh => Refusal::OrderPriceTooHigh,
            Rule::InvalidMailAddress => Refusal::InvalidMailAddress,
            Rule::MailSubjectTooLong => Refusal::MailSubjectTooLong,
            Rule::MailMessageTooLong => Refusal::MailMessageTooLong,
            Rule::InvalidPassword => Refusal::InvalidPassword,
            Rule::PasswordTooLong => Refusal::PasswordTooLong,
            Rule::InvalidUserCredentials => Refusal::InvalidUserCredentials,
            Rule::InvalidGroupCredentials => Refusal::InvalidGroupCredentials,
            Rule::InvalidWalletPassword => Refusal::InvalidWalletPassword,
            Rule::WalletPasswordTooLong => Refusal::WalletPasswordTooLong,
            Rule::InvalidWalletCredentials => Refusal::InvalidWalletCredentials,
            Rule::WalletCredentialsTooLong => Refusal::WalletCredentialsTooLong,
            Rule::InvalidSubwalletCredentials => Refusal::InvalidSubwalletCredentials,
            Rule::SubwalletCredentialsTooLong => Refusal::SubwalletCredentialsTooLong,
            Rule::CtfPasswordTooLong => Refusal::CtfPasswordTooLong,
            Rule::InvalidCtfPassword => Refusal::InvalidCtfPassword,
        }
    }
}
