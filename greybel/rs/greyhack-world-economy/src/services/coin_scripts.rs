//! What scripts do to coins through `CoinIntrinsics` and `BlockchainIntrinsics`: their checks and limits.

use greyhack_behaviour::{
    CoinLimit, check_new_subwallet, check_wallet_password, hash_password, is_valid_ip,
};
use greyhack_world_core::{CoinId, ComputerId, Result};

use crate::services::CoinService;
use crate::services::failure::EconomyFailure;
use crate::{Coin, SubWallet};

/// Maximum sub-wallets one wallet has for a coin (`Wallet.AddSubWallet`,
/// `BlockchainSystem.AddSubWallet`).
const MAX_SUBWALLETS_PER_COIN: usize = 10;

/// Whether a computer can reach the network (`Computer.IsNetworkActive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connectivity {
    /// The computer no longer exists.
    Missing,
    /// The computer cannot reach the network.
    Offline,
    /// The computer can reach the network.
    Online,
}

/// Methods of [`World`] for economy coin scripts.
impl CoinService {
    pub async fn connectivity(
        &self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
    ) -> Result<Connectivity> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Connectivity::Missing);
        }

        Ok(
            if self
                .networks
                .reach
                .is_network_active(computer, terminal_computers)
                .await?
            {
                Connectivity::Online
            } else {
                Connectivity::Offline
            },
        )
    }

    pub async fn set_coin_address(
        &self,
        coin: CoinId,
        address: &str,
    ) -> Result<std::result::Result<bool, EconomyFailure>> {
        // Resolve a network by public address, generating it if needed, or by domain.
        let has_network = if is_valid_ip(address) {
            self.network_generation
                .router_or_generate(&self.world, address)
                .await?
                .is_some()
        } else {
            self.networks.routers.lookup(address).await?.is_some()
        };

        if !has_network {
            return Ok(Err(EconomyFailure::UnknownAddress(address.to_owned())));
        }

        self.change_coin(coin, |coin| address.clone_into(&mut coin.address))
            .await
            .map(Ok)
    }

    pub async fn set_coin_reward(
        &self,
        coin: CoinId,
        reward: f64,
    ) -> Result<std::result::Result<bool, EconomyFailure>> {
        let reward = match CoinLimit::check_reward(reward) {
            Ok(reward) => reward,
            Err(CoinLimit::RewardTooHigh) => return Ok(Err(EconomyFailure::RewardTooHigh)),
            Err(_) => return Ok(Err(EconomyFailure::RewardTooLow)),
        };

        self.change_coin(coin, |coin| coin.reward = f64::from(reward))
            .await
            .map(Ok)
    }

    pub async fn set_coin_cycle_hours(
        &self,
        coin: CoinId,
        cycle_hours: f64,
    ) -> Result<std::result::Result<bool, EconomyFailure>> {
        let cycle_hours = match CoinLimit::check_cycle_hours(cycle_hours) {
            Ok(hours) => hours,
            Err(CoinLimit::CycleTooLong) => return Ok(Err(EconomyFailure::CycleTooLong)),
            Err(_) => return Ok(Err(EconomyFailure::CycleTooShort)),
        };

        self.change_coin(coin, |coin| coin.cycle_hours = f64::from(cycle_hours))
            .await
            .map(Ok)
    }

    pub async fn create_coin_subwallet(
        &self,
        coin: CoinId,
        wallet: &str,
        pin: &str,
        user: &str,
        password: &str,
    ) -> Result<std::result::Result<bool, EconomyFailure>> {
        let Some(owner) = self.wallets.wallet_by_user(wallet).await? else {
            return Ok(Err(EconomyFailure::UnknownWallet));
        };

        if self.coin(coin).await?.is_none() {
            return Ok(Ok(false));
        }

        if let Err(refusal) = check_new_subwallet(user, password) {
            return Ok(Err(EconomyFailure::Refusal(refusal)));
        }

        if self.wallets.subwallet(coin, user).await?.is_some() {
            return Ok(Err(EconomyFailure::SubwalletExists));
        }

        if owner.pin != pin {
            return Ok(Err(EconomyFailure::IncorrectPin));
        }

        let per_coin = self
            .wallets
            .subwallets(coin)
            .await?
            .iter()
            .filter(|subwallet| subwallet.wallet_user == wallet)
            .count();

        if per_coin >= MAX_SUBWALLETS_PER_COIN {
            return Ok(Err(EconomyFailure::TooManySubwallets));
        }

        self.wallets
            .add_subwallet(
                &SubWallet {
                    coin,
                    user: user.to_owned(),
                    password_hash: hash_password(password),
                    balance: 0.0,
                    info: String::new(),
                    wallet_user: wallet.to_owned(),
                },
                password,
            )
            .await?;

        Ok(Ok(true))
    }

    pub async fn reset_coin_password(
        &self,
        coin: CoinId,
        session_user: &str,
        password: &str,
    ) -> Result<std::result::Result<(), EconomyFailure>> {
        if let Err(refusal) = check_wallet_password(password) {
            return Ok(Err(EconomyFailure::Refusal(refusal)));
        }

        let Some(coin) = self.coin(coin).await? else {
            return Ok(Err(EconomyFailure::UnknownCoin(String::new())));
        };

        if coin.creator_user != session_user {
            return Ok(Err(EconomyFailure::NotOwner));
        }

        self.set_coin_password(coin.id, password).await?;

        Ok(Ok(()))
    }

    pub async fn login_coin(
        &self,
        name: &str,
        user: &str,
        password: &str,
    ) -> Result<Option<CoinId>> {
        Ok(self
            .coin_by_name(name)
            .await?
            .filter(|coin| coin.verify_creator(user, password))
            .map(|coin| coin.id))
    }

    pub async fn delete_coin_as_creator(
        &self,
        name: &str,
        user: &str,
        password: &str,
    ) -> Result<std::result::Result<(), EconomyFailure>> {
        let Some(coin) = self.coin_by_name(name).await? else {
            return Ok(Err(EconomyFailure::UnknownCoin(name.to_owned())));
        };

        if !coin.verify_creator(user, password) {
            return Ok(Err(EconomyFailure::IncorrectCredentials));
        }

        self.mine_jobs.delete_coin(name).await?;

        Ok(Ok(()))
    }

    pub async fn creator_coin_name(
        &self,
        session_user: &str,
        user: &str,
        password: &str,
    ) -> Result<std::result::Result<String, EconomyFailure>> {
        let Some(coin) = self
            .coins_of_creator(session_user)
            .await?
            .into_iter()
            .next()
        else {
            return Ok(Err(EconomyFailure::NoCoin));
        };

        if !coin.verify_creator(user, password) {
            return Ok(Err(EconomyFailure::IncorrectCredentials));
        }

        Ok(Ok(coin.name))
    }
}

impl CoinService {
    pub async fn change_coin(&self, coin: CoinId, change: impl FnOnce(&mut Coin)) -> Result<bool> {
        let Some(mut row) = self.coin(coin).await? else {
            return Ok(false);
        };

        change(&mut row);
        self.save_coin(&row).await?;

        Ok(true)
    }
}
