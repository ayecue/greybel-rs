//! Storage for order books.

use turso::params;

use super::super::Database;
use super::super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt};
use crate::{CoinId, Offer, Result, Side, WalletId};

const fn side_code(side: Side) -> i64 {
    match side {
        Side::Buy => 0,
        Side::Sell => 1,
    }
}

impl FromRow for Offer {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            coin: CoinId(row.get("coin_id")?),
            wallet: WalletId(row.get("wallet_id")?),
            subwallet: row.get("subwallet")?,
            side: if row.get::<i64>("sell")? == 0 {
                Side::Buy
            } else {
                Side::Sell
            },
            price: row.get("price")?,
            amount: row.get("amount")?,
        })
    }
}

impl Database {
    /// Returns `coin` offers on `side`, oldest first.
    pub(crate) async fn offers(&self, coin: CoinId, side: Side) -> Result<Vec<Offer>> {
        Ok(fetch_all(
            &self.conn,
            "SELECT coin_id, wallet_id, subwallet, sell, price, amount FROM offers WHERE coin_id = ?1 AND sell = ?2 ORDER BY id",
            params![coin.0, side_code(side)],
        )
        .await?)
    }

    /// Returns `wallet`'s offer on `coin` from either side.
    pub(crate) async fn offer_of(&self, coin: CoinId, wallet: WalletId) -> Result<Option<Offer>> {
        Ok(fetch_opt(
            &self.conn,
            "SELECT coin_id, wallet_id, subwallet, sell, price, amount FROM offers WHERE coin_id = ?1 AND wallet_id = ?2",
            params![coin.0, wallet.0],
        )
        .await?)
    }

    /// Returns all `wallet` offers in placement order.
    pub(crate) async fn offers_of_wallet(&self, wallet: WalletId) -> Result<Vec<Offer>> {
        Ok(fetch_all(
            &self.conn,
            "SELECT coin_id, wallet_id, subwallet, sell, price, amount FROM offers WHERE wallet_id = ?1 ORDER BY id",
            params![wallet.0],
        )
        .await?)
    }

    /// Places `offer` after existing offers on its coin and side.
    pub(crate) async fn insert_offer(&self, offer: &Offer) -> Result<()> {
        execute(
            &self.conn,
            "INSERT INTO offers (coin_id, wallet_id, subwallet, sell, price, amount) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                offer.coin.0,
                offer.wallet.0,
                offer.subwallet.as_str(),
                side_code(offer.side),
                offer.price,
                offer.amount,
            ],
        )
        .await?;

        Ok(())
    }

    /// Updates offer price and amount.
    pub(crate) async fn save_offer(&self, offer: &Offer) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE offers SET price = ?3, amount = ?4 WHERE coin_id = ?1 AND wallet_id = ?2",
            params![offer.coin.0, offer.wallet.0, offer.price, offer.amount],
        )
        .await?;

        Ok(())
    }

    /// Removes `wallet`'s offer on `coin` and returns whether one existed.
    pub(crate) async fn remove_offer(&self, coin: CoinId, wallet: WalletId) -> Result<bool> {
        Ok(execute(
            &self.conn,
            "DELETE FROM offers WHERE coin_id = ?1 AND wallet_id = ?2",
            params![coin.0, wallet.0],
        )
        .await?
            > 0)
    }
}
