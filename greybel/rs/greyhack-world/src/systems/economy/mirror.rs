//! Copying an economy layout into the world.

use greyhack_behaviour::hash_password;

use crate::{Coin, CoinId, EconomyLayout, Offer, Result, SubWallet, World};

impl World {
    /// Mirrors global economy rows, resolving coin and wallet names to local IDs.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn mirror_economy(&mut self, layout: &EconomyLayout) -> Result<()> {
        let mut coins = std::collections::HashMap::new();
        let mut wallets = std::collections::HashMap::new();

        for bank in &layout.banks {
            self.add_bank(&bank.number, &bank.password, bank.money)
                .await?;
        }

        for subscription in &layout.subscriptions {
            self.db.save_subscription(subscription).await?;
        }

        for draft in &layout.coins {
            let id = self
                .db
                .insert_coin(
                    &Coin {
                        id: CoinId(0),
                        name: draft.name.clone(),
                        creator_user: draft.creator_user.clone(),
                        creator_password_hash: hash_password(&draft.creator_password),
                        address: draft.address.clone(),
                        cycle_hours: draft.cycle_hours,
                        last_cycle_at: None,
                        reward: draft.reward,
                        mined_coins: draft.mined_coins,
                        max_supply: draft.max_supply,
                        price: draft.price,
                    },
                    Some(&draft.creator_password),
                )
                .await?;

            coins.insert(draft.name.clone(), id);

            for price in &draft.prices {
                self.db.insert_price(id, price).await?;
            }
        }

        for draft in &layout.wallets {
            let id = self
                .add_wallet(&draft.user, &draft.password, &draft.pin, &draft.bank)
                .await?;

            wallets.insert(draft.user.clone(), id);
        }

        for draft in &layout.wallets {
            for subwallet in &draft.subwallets {
                let coin = coins[&subwallet.coin];
                let wallet = wallets[&subwallet.wallet_user];

                self.add_subwallet(
                    &SubWallet {
                        coin,
                        user: subwallet.user.clone(),
                        password_hash: hash_password(&subwallet.password),
                        balance: subwallet.balance,
                        info: subwallet.info.clone(),
                        wallet_user: subwallet.wallet_user.clone(),
                    },
                    &subwallet.password,
                )
                .await?;
                self.add_balance(wallet, &subwallet.coin, subwallet.balance)
                    .await?;
            }
        }

        for offer in &layout.offers {
            self.db
                .insert_offer(&Offer {
                    coin: coins[&offer.coin],
                    wallet: wallets[&offer.wallet_user],
                    subwallet: offer.subwallet.clone(),
                    side: offer.side,
                    price: offer.price,
                    amount: offer.amount,
                })
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/mirror.rs"]
mod tests;
