//! Coin prices, trades, and offers (`BlockchainSystem.UpdateAllPrices`, `Coin.UpdateMoneyValue`).

use greyhack_behaviour::{PRICE_HISTORY_LIMIT, order_book_price, pending_trade_price};

use crate::{CoinId, Offer, PriceRecord, Result, Side, WalletId, World};

impl World {
    /// Returns recorded `coin` prices oldest first.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn price_history(&self, coin: CoinId) -> Result<Vec<PriceRecord>> {
        self.db.price_history(coin).await
    }

    /// Adds a matched trade of `amount` coins at `price` to the pending `coin` price update.
    /// Zero-price or zero-amount trades have no effect (`Coin.AddTransaction`).
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn record_trade(&mut self, coin: CoinId, price: i64, amount: i64) -> Result<()> {
        if price <= 0 || amount <= 0 {
            return Ok(());
        }

        self.db.record_trade(coin, price, amount).await
    }

    /// Returns the visible `coin` price (`Coin.GetVisibleMoneyValue`): pending-trade average,
    /// current price, then order-book midpoint. Returns `None` if the coin is absent.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn visible_price(&self, coin: CoinId) -> Result<Option<f64>> {
        let Some(data) = self.db.coin(coin).await? else {
            return Ok(None);
        };
        let (value, amount) = self.db.pending_trades(coin).await?;
        let trade_price = pending_trade_price(value, amount);

        if trade_price > 0 {
            return Ok(Some(trade_price as f64));
        }

        if data.price > 0.0 {
            return Ok(Some(data.price));
        }

        Ok(Some(self.order_book_price(coin).await?))
    }

    /// Returns the midpoint of best buy and sell offers, or the available side's price
    /// (`Coin.GetOrderBookValue`).
    async fn order_book_price(&self, coin: CoinId) -> Result<f64> {
        let valid = |offer: &Offer| offer.amount > 0.0 && offer.price > 0.0;
        let best_buy = self
            .db
            .offers(coin, Side::Buy)
            .await?
            .iter()
            .filter(|offer| valid(offer))
            .map(|offer| offer.price as i64)
            .max()
            .unwrap_or(0);
        let best_sell = self
            .db
            .offers(coin, Side::Sell)
            .await?
            .iter()
            .filter(|offer| valid(offer))
            .map(|offer| offer.price as i64)
            .min()
            .unwrap_or(0);

        Ok(order_book_price(best_buy, best_sell))
    }

    /// Runs daily server upkeep: updates coin prices from daily trades
    /// (`BlockchainSystem.UpdateAllPrices`) and charges due monthly subscriptions (`BankSubs`). The
    /// server invokes this when the world clock crosses a day boundary.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn daily_upkeep(&mut self) -> Result<()> {
        self.update_prices().await?;

        self.run_subscription_payments().await
    }

    /// Sets each coin with new trades to its amount-weighted average trade price, retains seven
    /// price records, and clears pending trades (`Coin.UpdateMoneyValue`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn update_prices(&mut self) -> Result<()> {
        for mut coin in self.db.coins().await? {
            let trades = self.db.pending_trades(coin.id).await?;

            if trades.1 <= 0 {
                continue;
            }

            let new_price = pending_trade_price(trades.0, trades.1);

            if coin.price != new_price as f64 {
                coin.price = new_price as f64;
                self.db.save_coin(&coin).await?;
                self.record_price(
                    coin.id,
                    &PriceRecord {
                        price: coin.price,
                        date: self.date(),
                    },
                    PRICE_HISTORY_LIMIT,
                )
                .await?;
            }

            self.db.clear_pending_trades(coin.id).await?;
        }

        Ok(())
    }

    /// Appends `record` to `coin` prices and removes older records beyond `limit`.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub(crate) async fn record_price(
        &mut self,
        coin: CoinId,
        record: &PriceRecord,
        limit: usize,
    ) -> Result<()> {
        self.db.insert_price(coin, record).await?;

        while self.db.price_history(coin).await?.len() > limit {
            self.db.remove_oldest_price(coin).await?;
        }

        Ok(())
    }

    /// Returns `coin` offers on `side`, oldest first.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn offers(&self, coin: CoinId, side: Side) -> Result<Vec<Offer>> {
        self.db.offers(coin, side).await
    }

    /// Returns all `wallet` offers in placement order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn offers_of_wallet(&self, wallet: WalletId) -> Result<Vec<Offer>> {
        self.db.offers_of_wallet(wallet).await
    }
}

#[cfg(test)]
#[path = "tests/prices.rs"]
mod tests;
