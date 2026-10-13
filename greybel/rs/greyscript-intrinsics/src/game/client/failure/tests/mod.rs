//! Checks that fixed Grey Hack texts state the same limits as the rules in `greyhack-behaviour`.

#[path = "support/mod.rs"]
mod support;

use greyhack_behaviour::{
    MAX_CONTENT, MAX_CREDENTIAL, MAX_CTF_PASSWORD, MAX_NAME, MAX_ORDER_COINS, MAX_ORDER_PRICE,
    MAX_WALLET_CREDENTIAL, Refusal,
};

use support::text;

#[test]
fn should_state_the_name_and_content_limits() {
    assert_eq!(
        text(&Refusal::LongName),
        format!("Error: name cannot exceed the limit of {MAX_NAME} characters.")
    );
    assert_eq!(
        text(&Refusal::ContentTooLong),
        format!(
            "I can't save the file. The maximum of {},{:03} characters has been exceeded.",
            MAX_CONTENT / 1000,
            MAX_CONTENT % 1000
        )
    );
}

#[test]
fn should_state_the_order_limits() {
    assert_eq!(
        text(&Refusal::TooManyOrderCoins),
        format!("Error: Max coins per trade is {MAX_ORDER_COINS}")
    );
    assert_eq!(
        text(&Refusal::OrderPriceTooHigh),
        format!("Error: Max unit price is ${MAX_ORDER_PRICE}")
    );
}

#[test]
fn should_state_the_credential_limits() {
    assert_eq!(
        text(&Refusal::PasswordTooLong),
        format!("Error: the password cannot exceed the limit of {MAX_CREDENTIAL} characters.")
    );
    assert_eq!(
        text(&Refusal::SubwalletCredentialsTooLong),
        format!(
            "create_subwallet: username and password cannot exceed the {MAX_CREDENTIAL} character limit."
        )
    );
    assert_eq!(
        text(&Refusal::WalletPasswordTooLong),
        format!(
            "reset_password: name and password cannot exceed the {MAX_WALLET_CREDENTIAL} character limit."
        )
    );
    assert_eq!(
        text(&Refusal::WalletCredentialsTooLong),
        format!(
            "create_wallet: name and password cannot exceed the {MAX_WALLET_CREDENTIAL} character limit."
        )
    );
    assert_eq!(
        text(&Refusal::CtfPasswordTooLong),
        format!(
            "reset_ctf_password: password can't exceed the maximum limit of {MAX_CTF_PASSWORD} characters"
        )
    );
}
