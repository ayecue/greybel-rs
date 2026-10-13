//! Coin order-book matching and bank-settled wallet trades.

use greyhack_behaviour::{check_order, clamp_balance};

use crate::model::{CoinId, MarketFailure, Offer, Side, Trade, WalletId};
use crate::{Result, World};

impl World {
    /// Places a buy order for `amount` of `coin` at `price`, matching sells at or below that price.
    /// Returns execution or failure; `None` if the wallet is absent.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn buy_coin(
        &mut self,
        coin: &str,
        amount: f64,
        price: f64,
        buyer: WalletId,
        subwallet: &str,
    ) -> Result<Option<Result<Trade, MarketFailure>>> {
        let Some(coin_id) = self.coin_id(coin).await? else {
            return Ok(Some(Err(MarketFailure::UnknownCoin(coin.to_owned()))));
        };

        if let Err(refusal) = check_order(amount, price) {
            return Ok(Some(Err(MarketFailure::Refusal(refusal))));
        }

        let Some((user, bank)) = self.trader(buyer).await? else {
            return Ok(None);
        };

        if self.db.balance(buyer, coin_id).await?.is_none() {
            let address = self
                .coin(coin_id)
                .await?
                .map(|coin| coin.address)
                .unwrap_or_default();

            return Ok(Some(Err(MarketFailure::MissingRegistration {
                coin: coin.to_owned(),
                address,
            })));
        }

        if self.db.subwallet(coin_id, subwallet).await?.is_none() {
            return Ok(Some(Err(MarketFailure::MissingSubwallet(
                subwallet.to_owned(),
            ))));
        }

        let cost = amount * price;

        match self.db.bank(&bank).await? {
            None => {
                return Ok(Some(Err(MarketFailure::NoBankAccount)));
            }
            Some(account) if account.money < cost => {
                return Ok(Some(Err(MarketFailure::InsufficientMoney(cost as i64))));
            }
            Some(_) => {}
        }

        let prior = self
            .db
            .offer_of(coin_id, buyer)
            .await?
            .map(|offer| offer.side);

        if let Some(offer) = prior {
            return Ok(Some(Err(MarketFailure::OpenOffer {
                offer,
                order: Side::Buy,
            })));
        }

        Ok(Some(Ok(self
            .trade(
                coin_id,
                coin,
                Side::Buy,
                (buyer, user, bank),
                Order {
                    amount,
                    price,
                    subwallet,
                },
            )
            .await?)))
    }

    /// Places a sell order for `amount` of `coin` at `price`, matching buys at or above that price.
    /// Returns execution or failure; `None` if the wallet is absent.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn sell_coin(
        &mut self,
        coin: &str,
        amount: f64,
        price: f64,
        seller: WalletId,
        subwallet: &str,
    ) -> Result<Option<Result<Trade, MarketFailure>>> {
        let Some(coin_id) = self.coin_id(coin).await? else {
            return Ok(Some(Err(MarketFailure::UnknownCoin(coin.to_owned()))));
        };

        if let Err(refusal) = check_order(amount, price) {
            return Ok(Some(Err(MarketFailure::Refusal(refusal))));
        }

        let Some(wallet) = self.wallet(seller).await? else {
            return Ok(None);
        };
        let (user, bank) = (wallet.user, wallet.bank);
        let Some(account) = self.db.subwallet(coin_id, subwallet).await? else {
            return Ok(Some(Err(MarketFailure::MissingSubwallet(
                subwallet.to_owned(),
            ))));
        };

        if account.balance < amount {
            return Ok(Some(Err(MarketFailure::InsufficientCoins)));
        }

        if self.db.bank(&bank).await?.is_none() {
            return Ok(Some(Err(MarketFailure::NoBankAccount)));
        }

        if let Some(offer) = self.db.offer_of(coin_id, seller).await? {
            return Ok(Some(Err(MarketFailure::OpenOffer {
                offer: offer.side,
                order: Side::Sell,
            })));
        }

        Ok(Some(Ok(self
            .trade(
                coin_id,
                coin,
                Side::Sell,
                (seller, user, bank),
                Order {
                    amount,
                    price,
                    subwallet,
                },
            )
            .await?)))
    }

    /// Cancels `wallet`'s open order on `coin` and returns whether one existed.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn cancel_trade(&mut self, coin: &str, wallet: WalletId) -> Result<bool> {
        let Some(id) = self.coin_id(coin).await? else {
            return Ok(false);
        };

        if self.trader(wallet).await?.is_none() {
            return Ok(false);
        }

        self.db.remove_offer(id, wallet).await
    }

    /// Returns the user and bank account for wallet `id`.
    async fn trader(&self, id: WalletId) -> Result<Option<(String, String)>> {
        Ok(self
            .wallet(id)
            .await?
            .map(|wallet| (wallet.user, wallet.bank)))
    }

    /// Matches `trader` (wallet, user, bank) against acceptable offers oldest first and queues any
    /// remainder.
    async fn trade(
        &mut self,
        coin_id: CoinId,
        coin: &str,
        side: Side,
        trader: (WalletId, String, String),
        order: Order<'_>,
    ) -> Result<Trade> {
        let Order {
            amount,
            price,
            subwallet,
        } = order;
        let (trader, _, bank) = trader;
        let mut unmatched_amount = amount;
        let mut matched_amount = 0.0;

        loop {
            let offers = self.db.offers(coin_id, side.other()).await?;
            let Some(mut offer) = offers
                .into_iter()
                .find(|offer| side.accepts(offer.price, price) && offer.amount > 0.0)
            else {
                break;
            };
            let Some(other) = self.wallet(offer.wallet).await? else {
                self.db.remove_offer(coin_id, offer.wallet).await?;
                continue;
            };
            let trade_amount = unmatched_amount.min(offer.amount);
            let offer_account = self.db.subwallet(coin_id, &offer.subwallet).await?;
            // As in `BlockchainSystem.SellCoin`/`BuyCoin`, offers are invalid if their sub-wallet
            // is missing, owner cannot pay, or balance is below the offered amount.
            let affordable = match (side, &offer_account) {
                (_, None) => false,
                (Side::Sell, Some(_)) => self
                    .db
                    .bank(&other.bank)
                    .await?
                    .is_some_and(|bank| bank.money >= trade_amount * offer.price),
                (Side::Buy, Some(account)) => account.balance >= offer.amount,
            };

            if !affordable {
                self.db.remove_offer(coin_id, offer.wallet).await?;
                continue;
            }

            let ((buyer, buyer_bank), (seller, seller_bank)) = match side {
                Side::Buy => ((trader, &bank), (other.id, &other.bank)),
                Side::Sell => ((other.id, &other.bank), (trader, &bank)),
            };

            self.transfer_money(buyer_bank, seller_bank, trade_amount * offer.price)
                .await?;
            self.add_balance(seller, coin, -trade_amount).await?;
            self.add_balance(buyer, coin, trade_amount).await?;

            let delta = if side == Side::Buy {
                trade_amount
            } else {
                -trade_amount
            };

            if let Some(mut account) = self.db.subwallet(coin_id, subwallet).await? {
                account.balance = clamp_balance(account.balance + delta);
                self.db.save_subwallet(&account).await?;
            }

            // The offer's sub-wallet takes the opposite side (`BlockchainSystem.AutoTransaction`).
            if let Some(mut account) = offer_account {
                account.balance = clamp_balance(account.balance - delta);
                self.db.save_subwallet(&account).await?;
            }

            // Every match contributes to the next daily coin price
            // (`BlockchainSystem.AutoTransaction`).
            self.record_trade(coin_id, offer.price as i64, trade_amount as i64)
                .await?;
            unmatched_amount -= trade_amount;
            matched_amount += trade_amount;

            if offer.amount - trade_amount <= 0.0 {
                self.db.remove_offer(coin_id, offer.wallet).await?;
            } else {
                offer.amount -= trade_amount;
                self.db.save_offer(&offer).await?;
            }

            if unmatched_amount <= 0.0 {
                return Ok(Trade::Complete);
            }
        }

        self.db
            .insert_offer(&Offer {
                coin: coin_id,
                wallet: trader,
                subwallet: subwallet.to_owned(),
                side,
                price,
                amount: unmatched_amount,
            })
            .await?;

        Ok(if matched_amount <= 0.0 {
            Trade::Pending
        } else {
            Trade::Partial
        })
    }
}

/// Order amount, unit price, and credited or debited sub-wallet.
struct Order<'a> {
    amount: f64,
    price: f64,
    subwallet: &'a str,
}

#[cfg(test)]
#[path = "tests/market.rs"]
mod tests;
