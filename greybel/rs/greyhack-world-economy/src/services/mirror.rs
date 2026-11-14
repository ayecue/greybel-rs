//! Copying an economy layout into the world.

use greyhack_behaviour::hash_password;

use crate::services::Bank;
use crate::{Coin, CoinId, EconomyLayout, Offer, Result, SubWallet};

/// Methods of [`World`] for economy mirror.
impl Bank {
    pub async fn mirror_economy(&self, layout: &EconomyLayout) -> Result<()> {
        let mut coins = std::collections::HashMap::new();
        let mut wallets = std::collections::HashMap::new();

        for bank in &layout.banks {
            self.add_bank(&bank.number, &bank.password, bank.money)
                .await?;
        }

        for subscription in &layout.subscriptions {
            self.subscriptions.save_subscription(subscription).await?;
            self.schedule_payment(subscription).await?;
        }

        for draft in &layout.coins {
            let id = self
                .coins
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
                self.coins.insert_price(id, price).await?;
            }
        }

        for draft in &layout.wallets {
            let id = self
                .wallets
                .add_wallet(&draft.user, &draft.password, &draft.pin, &draft.bank)
                .await?;

            wallets.insert(draft.user.clone(), id);
        }

        for draft in &layout.wallets {
            for subwallet in &draft.subwallets {
                let coin = coins[&subwallet.coin];
                let wallet = wallets[&subwallet.wallet_user];

                self.wallets
                    .add_subwallet(
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
                self.wallets
                    .add_balance(wallet, &subwallet.coin, subwallet.balance)
                    .await?;
            }
        }

        for offer in &layout.offers {
            self.offers
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
