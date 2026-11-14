//! Conversions of economy entities and failures into world-protocol values.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_router::protocol::*;

impl IntoProtocol for crate::MarketFailure {
    type Output = MarketFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::UnknownCoin(coin) => MarketFailure::UnknownCoin(coin),
            Self::Refusal(refusal) => MarketFailure::Refusal(refusal.into_protocol()),
            Self::MissingRegistration { coin, address } => {
                MarketFailure::MissingRegistration { coin, address }
            }
            Self::MissingSubwallet(subwallet) => MarketFailure::MissingSubwallet(subwallet),
            Self::NoBankAccount => MarketFailure::NoBankAccount,
            Self::InsufficientMoney(cost) => MarketFailure::InsufficientMoney(cost),
            Self::InsufficientCoins => MarketFailure::InsufficientCoins,
            Self::OpenOffer { offer, order } => MarketFailure::OpenOffer {
                offer: offer.into_protocol(),
                order: order.into_protocol(),
            },
        }
    }
}

impl IntoProtocol for crate::TransferFailure {
    type Output = TransferFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::UnknownCoin => TransferFailure::UnknownCoin,
            Self::MissingSubwallet(subwallet) => TransferFailure::MissingSubwallet(subwallet),
            Self::NoCoins(subwallet) => TransferFailure::NoCoins(subwallet),
            Self::InsufficientCoins => TransferFailure::InsufficientCoins,
            Self::SaleOfferShortfall => TransferFailure::SaleOfferShortfall,
        }
    }
}

impl IntoProtocol for crate::MineJobFailure {
    type Output = MineJobFailure;

    fn into_protocol(self) -> Self::Output {
        match self {
            Self::ComputerUnavailable => MineJobFailure::ComputerUnavailable,
            Self::CoinGone => MineJobFailure::CoinGone,
            Self::AllMined(coin) => MineJobFailure::AllMined(coin),
            Self::SubwalletGone => MineJobFailure::SubwalletGone,
            Self::SecondMineJob => MineJobFailure::SecondMineJob,
            Self::DamagedGpu => MineJobFailure::DamagedGpu,
            Self::CorruptedGpu => MineJobFailure::CorruptedGpu,
        }
    }
}

impl IntoProtocol for crate::EconomyFailure {
    type Output = Failure;

    fn into_protocol(self) -> Self::Output {
        Failure::Economy(match self {
            Self::NoInternet => EconomyFailure::NoInternet,
            Self::ComputerUnavailable => EconomyFailure::ComputerUnavailable,
            Self::UnknownCoin(name) => EconomyFailure::UnknownCoin(name),
            Self::IncorrectCredentials => EconomyFailure::IncorrectCredentials,
            Self::NoCoin => EconomyFailure::NoCoinCreated,
            Self::NoBankAccount => EconomyFailure::NoBankAccount,
            Self::WalletUserExists => EconomyFailure::WalletUserTaken,
            Self::OneWalletPerPlayer => EconomyFailure::OneWalletPerPlayer,
            Self::UnknownWallet => EconomyFailure::UnknownWallet,
            Self::NotOwner => EconomyFailure::NotOwner,
            Self::PasswordResetTooSoon => EconomyFailure::PasswordResetTooSoon,
            Self::SubwalletExists => EconomyFailure::SubwalletTaken,
            Self::IncorrectPin => EconomyFailure::IncorrectPin,
            Self::TooManySubwallets => EconomyFailure::TooManySubwallets,
            Self::UnknownAddress(address) => EconomyFailure::UnknownAddress(address),
            Self::RewardTooHigh => EconomyFailure::RewardTooHigh,
            Self::RewardTooLow => EconomyFailure::RewardTooLow,
            Self::CycleTooLong => EconomyFailure::CycleTooLong,
            Self::CycleTooShort => EconomyFailure::CycleTooShort,
            Self::Refusal(refusal) => return Failure::Refusal(refusal.into_protocol()),
        })
    }
}
