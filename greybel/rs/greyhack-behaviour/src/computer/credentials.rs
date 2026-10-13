//! Limits, validation, and hashing for account names and passwords.

use md5::{Digest, Md5};

use crate::base::refusal::{Credential, Refusal, RuleResult};
use crate::computer::computers::MAX_USERS;
use crate::path;

pub const ROOT_USER: &str = "root";
pub const GUEST_USER: &str = "guest";
/// Maximum length of a user name, group name, or password.
pub const MAX_CREDENTIAL: usize = 15;
/// Maximum length of a wallet user name or password.
pub const MAX_WALLET_CREDENTIAL: usize = 16;
/// Maximum length of a CTF account password.
pub const MAX_CTF_PASSWORD: usize = 32;
/// Number of failed debug-account logins before access is blocked.
pub const MAX_DEBUG_LOGIN_ATTEMPTS: u32 = 3;

/// Credential validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialError {
    InvalidCharacters,
    TooLong,
}

/// Validates a regular user name, group name, or password.
pub fn validate_credential(credential: &str) -> Result<(), CredentialError> {
    validate(credential, MAX_CREDENTIAL, path::is_alphanumeric)
}

/// Validates wallet credentials.
pub fn validate_wallet_credential(credential: &str) -> Result<(), CredentialError> {
    validate(credential, MAX_WALLET_CREDENTIAL, path::is_valid_name)
}

fn validate(
    credential: &str,
    limit: usize,
    valid_characters: fn(&str) -> bool,
) -> Result<(), CredentialError> {
    if !valid_characters(credential) {
        Err(CredentialError::InvalidCharacters)
    } else if credential.chars().count() > limit {
        Err(CredentialError::TooLong)
    } else {
        Ok(())
    }
}

/// Validates a new password for `change_password`, `encrypt`, or `decrypt`.
pub fn check_password(password: &str) -> RuleResult {
    match validate_credential(password) {
        Ok(()) => Ok(()),
        Err(CredentialError::InvalidCharacters) => Err(Refusal::InvalidPassword),
        Err(CredentialError::TooLong) => Err(Refusal::PasswordTooLong),
    }
}

/// Validates the user name and password passed to `create_user`.
pub fn check_new_user(user: &str, password: &str) -> RuleResult {
    if user.is_empty() {
        Err(Refusal::InvalidArguments)
    } else if user.chars().count() > MAX_CREDENTIAL {
        Err(Refusal::TooLong(Credential::UserName))
    } else if password.chars().count() > MAX_CREDENTIAL {
        Err(Refusal::TooLong(Credential::Password))
    } else if !path::is_alphanumeric(user) || !path::is_alphanumeric(password) {
        Err(Refusal::InvalidUserCredentials)
    } else {
        Ok(())
    }
}

/// Validates the user and new group names passed to `create_group`.
pub fn check_new_group(user: &str, group: &str) -> RuleResult {
    if user.is_empty() || group.is_empty() {
        Err(Refusal::InvalidArguments)
    } else if group.chars().count() > MAX_CREDENTIAL {
        Err(Refusal::TooLong(Credential::GroupName))
    } else if !path::is_alphanumeric(user) || !path::is_alphanumeric(group) {
        Err(Refusal::InvalidGroupCredentials)
    } else {
        Ok(())
    }
}

/// Validates a new wallet or coin password for `reset_password`.
pub fn check_wallet_password(password: &str) -> RuleResult {
    match validate_wallet_credential(password) {
        Ok(()) => Ok(()),
        Err(CredentialError::InvalidCharacters) => Err(Refusal::InvalidWalletPassword),
        Err(CredentialError::TooLong) => Err(Refusal::WalletPasswordTooLong),
    }
}

/// Validates the user name and password passed to `create_wallet`.
pub fn check_new_wallet(user: &str, password: &str) -> RuleResult {
    let credentials = [user, password].map(validate_wallet_credential);

    if credentials.contains(&Err(CredentialError::InvalidCharacters)) {
        Err(Refusal::InvalidWalletCredentials)
    } else if credentials.contains(&Err(CredentialError::TooLong)) {
        Err(Refusal::WalletCredentialsTooLong)
    } else {
        Ok(())
    }
}

/// Validates `create_subwallet` credentials (`BlockchainSystem.ValidateSubWalletCredentials`).
pub fn check_new_subwallet(user: &str, password: &str) -> RuleResult {
    if !path::is_valid_name(user) || !path::is_valid_name(password) {
        Err(Refusal::InvalidSubwalletCredentials)
    } else if user.chars().count() > MAX_CREDENTIAL || password.chars().count() > MAX_CREDENTIAL {
        Err(Refusal::SubwalletCredentialsTooLong)
    } else {
        Ok(())
    }
}

/// Validates a new CTF account password (`PlayerIntrinsics.reset_ctf_password`, `OS.IsAlphaNumeric`).
pub fn check_ctf_password(password: &str) -> RuleResult {
    if password.chars().count() > MAX_CTF_PASSWORD {
        Err(Refusal::CtfPasswordTooLong)
    } else if password.is_empty() || !password.chars().all(|c| c.is_ascii_alphanumeric()) {
        Err(Refusal::InvalidCtfPassword)
    } else {
        Ok(())
    }
}

/// Why a new account cannot be created (`Computer.ValidateNewUser`, `ComputerIntrinsics.create_user`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountRefusal {
    /// The name is `root`, compared without letter case.
    RootExists,
    /// The name is `guest`, compared without letter case.
    ReservedGuest,
    /// The computer already has [`MAX_USERS`] accounts.
    TooManyUsers,
}

/// Rejects the reserved account names `root` and `guest`, compared without letter case
/// (`Computer.ValidateNewUser`).
pub fn check_account_name(name: &str) -> Result<(), AccountRefusal> {
    match name.to_lowercase().as_str() {
        ROOT_USER => Err(AccountRefusal::RootExists),
        GUEST_USER => Err(AccountRefusal::ReservedGuest),
        _ => Ok(()),
    }
}

/// Rejects a new account on a computer with `user_count` accounts (`ComputerIntrinsics.create_user`).
pub const fn check_account_capacity(user_count: usize) -> Result<(), AccountRefusal> {
    if user_count >= MAX_USERS {
        Err(AccountRefusal::TooManyUsers)
    } else {
        Ok(())
    }
}

/// Computes the password's lowercase hexadecimal MD5 (`PasswordsEncrypter.Encrypt`).
#[must_use]
pub fn hash_password(password: &str) -> String {
    format!("{:x}", Md5::digest(password.as_bytes()))
}

/// Formats account records as one `name:hash` line per user.
#[must_use]
pub fn passwd_content<'a>(accounts: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    accounts
        .into_iter()
        .map(|(name, hash)| format!("{name}:{hash}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Checks whether hashing `password` produces `hash`.
#[must_use]
pub fn verify_password(hash: &str, password: &str) -> bool {
    hash == hash_password(password)
}
