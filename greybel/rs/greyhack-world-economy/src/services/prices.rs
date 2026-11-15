//! Coin prices, trades, and offers (`BlockchainSystem.UpdateAllPrices`, `Coin.UpdateMoneyValue`).

use greyhack_behaviour::{PRICE_HISTORY_LIMIT, Side, order_book_price, pending_trade_price};
use greyhack_world_core::{CoinId, Result};

use crate::services::Market;
use crate::{Offer, PriceRecord, WalletId};

/// Methods of [`World`] for economy prices.
impl Market {
    pub async fn price_history(&self, coin: CoinId) -> Result<Vec<PriceRecord>> {
        self.coin_rows.price_history(coin).await
    }

    pub async fn record_trade(&self, coin: CoinId, price: i64, amount: i64) -> Result<()> {
        if price <= 0 || amount <= 0 {
            return Ok(());
        }

        self.coin_rows.record_trade(coin, price, amount).await
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "Coin.GetVisibleMoneyValue returns an integral price, which scripts see as an f64"
    )]
    pub async fn visible_price(&self, coin: CoinId) -> Result<Option<f64>> {
        let Some(data) = self.coins.coin(coin).await? else {
            return Ok(None);
        };
        let (value, amount) = self.coin_rows.pending_trades(coin).await?;
        let trade_price = pending_trade_price(value, amount);

        if trade_price > 0 {
            return Ok(Some(trade_price as f64));
        }

        if data.price > 0.0 {
            return Ok(Some(data.price));
        }

        Ok(Some(self.order_book_price(coin).await?))
    }

    pub async fn daily_upkeep(&self) -> Result<()> {
        self.update_prices().await
    }

    #[expect(
        clippy::float_cmp,
        reason = "BlockchainSystem.Coin.UpdateMoneyValue records only an exact price change"
    )]
    #[expect(
        clippy::cast_precision_loss,
        reason = "BlockchainSystem.Coin.UpdateMoneyValue stores its integral price as an f64"
    )]
    pub async fn update_prices(&self) -> Result<()> {
        for mut coin in self.coin_rows.coins().await? {
            let trades = self.coin_rows.pending_trades(coin.id).await?;

            if trades.1 <= 0 {
                continue;
            }

            let new_price = pending_trade_price(trades.0, trades.1);

            if coin.price != new_price as f64 {
                coin.price = new_price as f64;
                self.coin_rows.save_coin(&coin).await?;
                self.record_price(
                    coin.id,
                    &PriceRecord {
                        price: coin.price,
                        date: self.world.date(),
                    },
                    PRICE_HISTORY_LIMIT,
                )
                .await?;
            }

            self.coin_rows.clear_pending_trades(coin.id).await?;
        }

        Ok(())
    }

    pub async fn record_price(
        &self,
        coin: CoinId,
        record: &PriceRecord,
        limit: usize,
    ) -> Result<()> {
        self.coin_rows.insert_price(coin, record).await?;

        while self.coin_rows.price_history(coin).await?.len() > limit {
            self.coin_rows.remove_oldest_price(coin).await?;
        }

        Ok(())
    }

    pub async fn offers(&self, coin: CoinId, side: Side) -> Result<Vec<Offer>> {
        self.offers.offers(coin, side).await
    }

    pub async fn offers_of_wallet(&self, wallet: WalletId) -> Result<Vec<Offer>> {
        self.offers.offers_of_wallet(wallet).await
    }
}

impl Market {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Coin.GetOrderBookValue truncates offer prices like C# (int)"
    )]
    pub async fn order_book_price(&self, coin: CoinId) -> Result<f64> {
        let valid = |offer: &Offer| offer.amount > 0.0 && offer.price > 0.0;
        let best_buy = self
            .offers
            .offers(coin, Side::Buy)
            .await?
            .iter()
            .filter(|offer| valid(offer))
            .map(|offer| offer.price as i64)
            .max()
            .unwrap_or(0);
        let best_sell = self
            .offers
            .offers(coin, Side::Sell)
            .await?
            .iter()
            .filter(|offer| valid(offer))
            .map(|offer| offer.price as i64)
            .min()
            .unwrap_or(0);

        Ok(order_book_price(best_buy, best_sell))
    }
}
