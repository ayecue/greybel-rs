//! Grey Hack text for rule-rejected arguments.

use greyhack_behaviour::{Credential, FileOwnership, NameKind, Refusal};

use crate::error::IntrinsicError;

/// Converts `refusal` into `intrinsic`'s return text or raised error, selected by [`Refusal::is_raised`].
pub(in crate::game) fn refusal_text(
    intrinsic: &'static str,
    refusal: &Refusal,
) -> Result<String, IntrinsicError> {
    Ok(match refusal {
        Refusal::InvalidName(NameKind::File) => {
            "Error: only alphanumeric allowed as file name.".to_owned()
        }
        Refusal::InvalidName(NameKind::Folder) => {
            "Error: only alphanumeric allowed as folder name.".to_owned()
        }
        Refusal::InvalidName(NameKind::New) => {
            "Error: only alphanumeric allowed as newname".to_owned()
        }
        Refusal::LongName => "Error: name cannot exceed the limit of 128 characters.".to_owned(),
        Refusal::ContentTooLong => {
            "I can't save the file. The maximum of 160,000 characters has been exceeded.".to_owned()
        }
        // Preserve Grey Hack's misspelling of "process" and its message claiming 28 characters for the actual 24-character limit.
        Refusal::InvalidProcessName => {
            "Error: only alphanumeric allowed as proccess name.".to_owned()
        }
        Refusal::ProcessNameTooLong => {
            "Error: proccess name cannot exceed the limit of 28 characters.".to_owned()
        }
        Refusal::ReservedProcessName(name) => format!("Error: {name} is a reserved process name"),
        Refusal::EmptyOrder => {
            format!("{intrinsic}: coin amount and proposed price cannot be 0.")
        }
        Refusal::TooManyOrderCoins => "Error: Max coins per trade is 50000".to_owned(),
        Refusal::OrderPriceTooHigh => "Error: Max unit price is $10000".to_owned(),
        Refusal::InvalidMailAddress => "Invalid email address".to_owned(),
        Refusal::MailSubjectTooLong => "Mail subject too large".to_owned(),
        Refusal::MailMessageTooLong => "Mail message too large".to_owned(),
        Refusal::InvalidPassword => "Error: only alphanumeric allowed as password.".to_owned(),
        Refusal::PasswordTooLong => {
            "Error: the password cannot exceed the limit of 15 characters.".to_owned()
        }
        Refusal::InvalidUserCredentials => {
            "Error: only alphanumeric allowed as user name and password.".to_owned()
        }
        Refusal::InvalidGroupCredentials => {
            "Error: only alphanumeric allowed as user and group names.".to_owned()
        }
        Refusal::InvalidWalletPassword => {
            "reset_password: only alphanumeric characters are allowed as password".to_owned()
        }
        Refusal::WalletPasswordTooLong => {
            "reset_password: name and password cannot exceed the 16 character limit.".to_owned()
        }
        Refusal::InvalidWalletCredentials => {
            "Error: only alphanumeric allowed as name and password".to_owned()
        }
        Refusal::WalletCredentialsTooLong => {
            "create_wallet: name and password cannot exceed the 16 character limit.".to_owned()
        }
        Refusal::InvalidSubwalletCredentials => {
            "create_subwallet: only alphanumeric characters are allowed as username and password."
                .to_owned()
        }
        Refusal::SubwalletCredentialsTooLong => {
            "create_subwallet: username and password cannot exceed the 15 character limit."
                .to_owned()
        }
        Refusal::CtfPasswordTooLong => {
            "reset_ctf_password: password can't exceed the maximum limit of 32 characters"
                .to_owned()
        }
        Refusal::InvalidCtfPassword => {
            "reset_ctf_password: password must be alphanumeric".to_owned()
        }
        Refusal::InvalidArguments => return Err(IntrinsicError::InvalidArguments(intrinsic)),
        Refusal::TooLong(credential) => {
            return Err(IntrinsicError::TooLong(match credential {
                Credential::UserName => "username",
                Credential::Password => "password",
                Credential::GroupName => "groupname",
            }));
        }
        Refusal::NameTooLong => return Err(IntrinsicError::NameTooLong(intrinsic)),
        Refusal::FileOwnershipInvalidName(FileOwnership::Owner) => {
            return Err(IntrinsicError::InvalidOwner);
        }
        Refusal::FileOwnershipInvalidName(FileOwnership::Group) => {
            return Err(IntrinsicError::InvalidGroup);
        }
        Refusal::FileOwnershipNameTooLong(FileOwnership::Owner) => {
            return Err(IntrinsicError::OwnerTooLong);
        }
        Refusal::FileOwnershipNameTooLong(FileOwnership::Group) => {
            return Err(IntrinsicError::GroupTooLong);
        }
    })
}

/// Converts world-rule argument failures for `intrinsic` like [`refusal_text`].
pub(in crate::game) fn protocol_refusal_text(
    intrinsic: &'static str,
    refusal: &crate::protocol::Refusal,
) -> Result<String, IntrinsicError> {
    use crate::protocol::{
        Credential as ProtocolCredential, FileOwnership as ProtocolOwnership,
        NameKind as ProtocolName, Refusal as Protocol,
    };

    let ownership = |ownership: &ProtocolOwnership| match ownership {
        ProtocolOwnership::Owner => FileOwnership::Owner,
        ProtocolOwnership::Group => FileOwnership::Group,
    };
    let refusal = match refusal {
        Protocol::InvalidArguments => Refusal::InvalidArguments,
        Protocol::NameTooLong => Refusal::NameTooLong,
        Protocol::LongName => Refusal::LongName,
        Protocol::ContentTooLong => Refusal::ContentTooLong,
        Protocol::InvalidProcessName => Refusal::InvalidProcessName,
        Protocol::ProcessNameTooLong => Refusal::ProcessNameTooLong,
        Protocol::EmptyOrder => Refusal::EmptyOrder,
        Protocol::TooManyOrderCoins => Refusal::TooManyOrderCoins,
        Protocol::OrderPriceTooHigh => Refusal::OrderPriceTooHigh,
        Protocol::InvalidMailAddress => Refusal::InvalidMailAddress,
        Protocol::MailSubjectTooLong => Refusal::MailSubjectTooLong,
        Protocol::MailMessageTooLong => Refusal::MailMessageTooLong,
        Protocol::InvalidPassword => Refusal::InvalidPassword,
        Protocol::PasswordTooLong => Refusal::PasswordTooLong,
        Protocol::InvalidUserCredentials => Refusal::InvalidUserCredentials,
        Protocol::InvalidGroupCredentials => Refusal::InvalidGroupCredentials,
        Protocol::InvalidWalletPassword => Refusal::InvalidWalletPassword,
        Protocol::WalletPasswordTooLong => Refusal::WalletPasswordTooLong,
        Protocol::InvalidWalletCredentials => Refusal::InvalidWalletCredentials,
        Protocol::WalletCredentialsTooLong => Refusal::WalletCredentialsTooLong,
        Protocol::InvalidSubwalletCredentials => Refusal::InvalidSubwalletCredentials,
        Protocol::SubwalletCredentialsTooLong => Refusal::SubwalletCredentialsTooLong,
        Protocol::CtfPasswordTooLong => Refusal::CtfPasswordTooLong,
        Protocol::InvalidCtfPassword => Refusal::InvalidCtfPassword,
        Protocol::TooLong(credential) => Refusal::TooLong(match credential {
            ProtocolCredential::UserName => Credential::UserName,
            ProtocolCredential::Password => Credential::Password,
            ProtocolCredential::GroupName => Credential::GroupName,
        }),
        Protocol::FileOwnershipInvalidName(holder) => {
            Refusal::FileOwnershipInvalidName(ownership(holder))
        }
        Protocol::FileOwnershipNameTooLong(holder) => {
            Refusal::FileOwnershipNameTooLong(ownership(holder))
        }
        Protocol::InvalidName(kind) => Refusal::InvalidName(match kind {
            ProtocolName::File => NameKind::File,
            ProtocolName::Folder => NameKind::Folder,
            ProtocolName::New => NameKind::New,
        }),
        Protocol::ReservedProcessName(name) => Refusal::ReservedProcessName(name.clone()),
    };

    refusal_text(intrinsic, &refusal)
}
