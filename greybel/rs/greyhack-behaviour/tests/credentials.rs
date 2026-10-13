use greyhack_behaviour::{
    AccountRefusal, CredentialError, MAX_CREDENTIAL, MAX_USERS, MAX_WALLET_CREDENTIAL,
    check_account_capacity, check_account_name, validate_credential, validate_wallet_credential,
};

#[test]
fn should_validate_ordinary_credentials() {
    assert_eq!(validate_credential("abc123"), Ok(()));
    assert_eq!(
        validate_credential("not-valid"),
        Err(CredentialError::InvalidCharacters)
    );
    assert_eq!(
        validate_credential(&"a".repeat(MAX_CREDENTIAL + 1)),
        Err(CredentialError::TooLong)
    );
}

#[test]
fn should_validate_wallet_credentials() {
    assert_eq!(validate_wallet_credential("wallet_name"), Ok(()));
    assert_eq!(
        validate_wallet_credential("not valid"),
        Err(CredentialError::InvalidCharacters)
    );
    assert_eq!(
        validate_wallet_credential(&"a".repeat(MAX_WALLET_CREDENTIAL + 1)),
        Err(CredentialError::TooLong)
    );
}

#[test]
fn should_reserve_root_and_guest_without_letter_case() {
    assert_eq!(check_account_name("Root"), Err(AccountRefusal::RootExists));
    assert_eq!(
        check_account_name("GUEST"),
        Err(AccountRefusal::ReservedGuest)
    );
    assert_eq!(check_account_name("rooted"), Ok(()));
}

#[test]
fn should_refuse_accounts_beyond_the_user_limit() {
    assert_eq!(check_account_capacity(MAX_USERS - 1), Ok(()));
    assert_eq!(
        check_account_capacity(MAX_USERS),
        Err(AccountRefusal::TooManyUsers)
    );
}
