//! Grey Hack failure text for blockchains, coins, wallets, sub-wallets, and market operations.

use super::computer::COMPUTER_GONE;
use super::refusal::protocol_refusal_text;
use crate::error::IntrinsicError;
use crate::protocol::{EconomyFailure, MarketFailure, MineJobFailure, Side, TransferFailure};

/// Formats `failure` for `intrinsic` using `BlockchainIntrinsics`, `CoinIntrinsics`, `WalletIntrinsics`, or `SubWalletIntrinsics` messages.
pub(in crate::game) fn economy_text(intrinsic: &str, failure: &EconomyFailure) -> String {
    match failure {
        EconomyFailure::NoInternet => format!("{intrinsic}: No internet access."),
        EconomyFailure::ComputerUnavailable => COMPUTER_GONE.to_owned(),
        EconomyFailure::UnknownCoin(coin) => match intrinsic {
            "delete_coin" => format!("Error: {coin} does not exist"),
            "show_nodes" => "show_nodes: coin does not exist".to_owned(),
            "reset_password" => "Error: coin does not exist".to_owned(),
            "get_pending_trade" | "get_global_offers" | "cancel_pending_trade" => {
                "Error: Coin does not exist".to_owned()
            }
            _ => format!("{intrinsic}: Coin does not exist."),
        },
        EconomyFailure::IncorrectCredentials => match intrinsic {
            "login_wallet" => "login_wallet: Account not found or incorrect password".to_owned(),
            "delete_coin" => "Error: incorrect user/password".to_owned(),
            _ => format!("{intrinsic}: Incorrect user/password"),
        },
        EconomyFailure::NoCoinCreated => "get_coin_name: No currency has been found".to_owned(),
        EconomyFailure::NoBankAccount => {
            "Error: It is necessary to have a bank account to be able to create a wallet."
                .to_owned()
        }
        EconomyFailure::WalletUserTaken => "Error: Wallet user already exists.".to_owned(),
        EconomyFailure::OneWalletPerPlayer => {
            "Error: Only one wallet per player allowed.".to_owned()
        }
        EconomyFailure::UnknownWallet => match intrinsic {
            "show_nodes" => "show_nodes: wallet does not exist",
            "create_subwallet" => "Error: Wallet does not exists.",
            _ => "Error: Wallet does not exist.",
        }
        .to_owned(),
        EconomyFailure::NotOwner => {
            "Error: Only the account owner can change the password".to_owned()
        }
        EconomyFailure::PasswordResetTooSoon => {
            "Error: The password can only be reset once a day".to_owned()
        }
        EconomyFailure::NoPendingTrades => "No pending trades found".to_owned(),
        EconomyFailure::UnknownSubwallet => "get_balance: The account does not exist.".to_owned(),
        EconomyFailure::NoCoinBalance => {
            "wallet does not have coins in the selected currency".to_owned()
        }
        EconomyFailure::SubwalletTaken => "Error: subwallet already exists".to_owned(),
        EconomyFailure::IncorrectPin => "Error: Incorrect PIN".to_owned(),
        EconomyFailure::TooManySubwallets => {
            "Error: only a maximum of 10 subwallets per coin is allowed.".to_owned()
        }
        EconomyFailure::RewardTooHigh => {
            "Error: The reward cannot exceed 100000000 coins".to_owned()
        }
        EconomyFailure::RewardTooLow => "Error: The reward cannot be less than 1 coin".to_owned(),
        EconomyFailure::CycleTooLong => {
            "the mining cycle cannot exceed the maximum of 2160 hours".to_owned()
        }
        EconomyFailure::CycleTooShort => "the mining cycle cannot be less than 1 hour".to_owned(),
        EconomyFailure::UnknownAddress(address) => {
            format!("set_address: address {address}not found")
        }
        EconomyFailure::Transfer(failure) => transfer_text(failure),
        EconomyFailure::MineJob(failure) => mine_job_text(failure),
    }
}

/// Returns a coin method's precondition-failure status (`CoinIntrinsics.CheckStatus`).
pub(in crate::game) fn coin_status_text(failure: &EconomyFailure) -> String {
    match failure {
        EconomyFailure::NoInternet => "error: No internet access.".to_owned(),
        failure => economy_text("set_address", failure),
    }
}

/// Formats `CoinIntrinsics.transaction` failures.
fn transfer_text(failure: &TransferFailure) -> String {
    match failure {
        TransferFailure::UnknownCoin => "Error: coin does not exist".to_owned(),
        TransferFailure::MissingSubwallet(subwallet) => format!("Error: {subwallet} not found"),
        TransferFailure::NoCoins(subwallet) => {
            format!("Error: subwallet {subwallet} does not have coins in the selected currency")
        }
        TransferFailure::InsufficientCoins => {
            "Error: insufficient funds to complete the transaction".to_owned()
        }
        TransferFailure::SaleOfferShortfall => {
            "Error: coins are committed to a pending sale. Cancel it before transferring."
                .to_owned()
        }
    }
}

/// Formats `SubWalletIntrinsics.mining` failures.
fn mine_job_text(failure: &MineJobFailure) -> String {
    match failure {
        MineJobFailure::ComputerUnavailable => "mining: computer not found".to_owned(),
        MineJobFailure::CoinGone => "Error: coin does not exist".to_owned(),
        MineJobFailure::AllMined(coin) => format!("all existing coins of {coin} have been mined."),
        MineJobFailure::SubwalletGone => "mining: The account does not exist.".to_owned(),
        MineJobFailure::SecondMineJob => "Only one mining process is allowed.".to_owned(),
        MineJobFailure::DamagedGpu => {
            "Error: GPU is damaged. Unable to start the mining process.".to_owned()
        }
        MineJobFailure::CorruptedGpu => "Error: GPU returned corrupted data while performing floating point tests. Unable to start the mining process.".to_owned(),
    }
}

/// Formats `buy_coin` or `sell_coin` failures for `intrinsic` (`BlockchainSystem.BuyCoin`, `SellCoin`).
pub(in crate::game) fn market_text(
    intrinsic: &'static str,
    failure: &MarketFailure,
) -> Result<String, IntrinsicError> {
    Ok(match failure {
        MarketFailure::UnknownCoin(coin) => format!("Error: Coin {coin} does not exist"),
        MarketFailure::Refusal(refusal) => protocol_refusal_text(intrinsic, refusal)?,
        MarketFailure::MissingRegistration { coin, address } => format!(
            "You need to be registered in {coin} to be able to buy this coin.\nVisit this address for more information: {address}"
        ),
        MarketFailure::MissingSubwallet(subwallet) => {
            format!("Subwallet {subwallet} does not exist")
        }
        MarketFailure::NoBankAccount => {
            "Purchase failed: It is necessary to create a bank account to make transactions"
                .to_owned()
        }
        MarketFailure::InsufficientMoney(cost) => {
            format!("Purchase failed: Insufficient money in the bank. Required ${cost}")
        }
        MarketFailure::InsufficientCoins => {
            "Error: subwallet does not contain enough coins to sell".to_owned()
        }
        MarketFailure::OpenOffer { offer, order } => match (offer, order) {
            (_, Side::Sell) => {
                "Error: You already have a pending sale. Cancel it to create a new sale."
            }
            (Side::Buy, Side::Buy) => {
                "Error: You already have a pending purchase. Cancel it to create a new purchase."
            }
            (Side::Sell, Side::Buy) => {
                "Error: You already have a pending sale. Cancel it to create a new purchase."
            }
        }
        .to_owned(),
    })
}
