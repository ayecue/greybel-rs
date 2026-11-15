use greyhack_world_programs::protocol::Refusal;

use super::IntoProtocol;

impl IntoProtocol for greyhack_behaviour::Refusal {
    type Output = Refusal;

    fn into_protocol(self) -> Self::Output {
        use greyhack_behaviour::Refusal as Rule;

        match self {
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
