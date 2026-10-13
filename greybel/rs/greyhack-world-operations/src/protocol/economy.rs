//! Conversions of coin, wallet, market, and bank failures.

use super::{EconomyFailure, Failure, MarketFailure, MineJobFailure, Side, TransferFailure};

impl From<greyhack_world::Side> for Side {
    fn from(side: greyhack_world::Side) -> Self {
        match side {
            greyhack_world::Side::Buy => Self::Buy,
            greyhack_world::Side::Sell => Self::Sell,
        }
    }
}

impl From<greyhack_world::MarketFailure> for MarketFailure {
    fn from(failure: greyhack_world::MarketFailure) -> Self {
        use greyhack_world::MarketFailure as World;

        match failure {
            World::UnknownCoin(coin) => Self::UnknownCoin(coin),
            World::Refusal(refusal) => Self::Refusal(refusal.into()),
            World::MissingRegistration { coin, address } => {
                Self::MissingRegistration { coin, address }
            }
            World::MissingSubwallet(subwallet) => Self::MissingSubwallet(subwallet),
            World::NoBankAccount => Self::NoBankAccount,
            World::InsufficientMoney(cost) => Self::InsufficientMoney(cost),
            World::InsufficientCoins => Self::InsufficientCoins,
            World::OpenOffer { offer, order } => Self::OpenOffer {
                offer: offer.into(),
                order: order.into(),
            },
        }
    }
}

impl From<greyhack_world::TransferFailure> for TransferFailure {
    fn from(failure: greyhack_world::TransferFailure) -> Self {
        use greyhack_world::TransferFailure as World;

        match failure {
            World::UnknownCoin => Self::UnknownCoin,
            World::MissingSubwallet(subwallet) => Self::MissingSubwallet(subwallet),
            World::NoCoins(subwallet) => Self::NoCoins(subwallet),
            World::InsufficientCoins => Self::InsufficientCoins,
            World::SaleOfferShortfall => Self::SaleOfferShortfall,
        }
    }
}

impl From<greyhack_world::MineJobFailure> for MineJobFailure {
    fn from(failure: greyhack_world::MineJobFailure) -> Self {
        use greyhack_world::MineJobFailure as World;

        match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::CoinGone => Self::CoinGone,
            World::AllMined(coin) => Self::AllMined(coin),
            World::SubwalletGone => Self::SubwalletGone,
            World::SecondMineJob => Self::SecondMineJob,
            World::DamagedGpu => Self::DamagedGpu,
            World::CorruptedGpu => Self::CorruptedGpu,
        }
    }
}

impl From<greyhack_world::EconomyFailure> for Failure {
    fn from(failure: greyhack_world::EconomyFailure) -> Self {
        use greyhack_world::EconomyFailure as World;

        Self::Economy(match failure {
            World::NoInternet => EconomyFailure::NoInternet,
            World::ComputerUnavailable => EconomyFailure::ComputerUnavailable,
            World::UnknownCoin(name) => EconomyFailure::UnknownCoin(name),
            World::IncorrectCredentials => EconomyFailure::IncorrectCredentials,
            World::NoCoin => EconomyFailure::NoCoinCreated,
            World::NoBankAccount => EconomyFailure::NoBankAccount,
            World::WalletUserExists => EconomyFailure::WalletUserTaken,
            World::OneWalletPerPlayer => EconomyFailure::OneWalletPerPlayer,
            World::UnknownWallet => EconomyFailure::UnknownWallet,
            World::NotOwner => EconomyFailure::NotOwner,
            World::PasswordResetTooSoon => EconomyFailure::PasswordResetTooSoon,
            World::SubwalletExists => EconomyFailure::SubwalletTaken,
            World::IncorrectPin => EconomyFailure::IncorrectPin,
            World::TooManySubwallets => EconomyFailure::TooManySubwallets,
            World::UnknownAddress(address) => EconomyFailure::UnknownAddress(address),
            World::RewardTooHigh => EconomyFailure::RewardTooHigh,
            World::RewardTooLow => EconomyFailure::RewardTooLow,
            World::CycleTooLong => EconomyFailure::CycleTooLong,
            World::CycleTooShort => EconomyFailure::CycleTooShort,
            World::Refusal(refusal) => return Self::Refusal(refusal.into()),
        })
    }
}
