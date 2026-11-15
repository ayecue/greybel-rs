//! Coin order-book matching and bank-settled wallet trades.

use greyhack_behaviour::{Side, check_order, clamp_balance};
use greyhack_world_core::{CoinId, Result};

use crate::model::{MarketFailure, Offer, Trade, WalletId};

/// Methods of [`World`] for economy market.
impl Market {
    pub async fn buy_coin(
        &self,
        coin: &str,
        amount: f64,
        price: f64,
        buyer: WalletId,
        subwallet: &str,
    ) -> Result<Option<Result<Trade, MarketFailure>>> {
        let Some(coin_id) = self.coins.coin_id(coin).await? else {
            return Ok(Some(Err(MarketFailure::UnknownCoin(coin.to_owned()))));
        };

        if let Err(refusal) = check_order(amount, price) {
            return Ok(Some(Err(MarketFailure::Refusal(refusal))));
        }

        let Some((user, bank)) = self.trader(buyer).await? else {
            return Ok(None);
        };

        if self.wallet_rows.balance(buyer, coin_id).await?.is_none() {
            let address = self
                .coins
                .coin(coin_id)
                .await?
                .map(|coin| coin.address)
                .unwrap_or_default();

            return Ok(Some(Err(MarketFailure::MissingRegistration {
                coin: coin.to_owned(),
                address,
            })));
        }

        if self
            .subwallets
            .subwallet(coin_id, subwallet)
            .await?
            .is_none()
        {
            return Ok(Some(Err(MarketFailure::MissingSubwallet(
                subwallet.to_owned(),
            ))));
        }

        let cost = amount * price;

        match self.banks.bank(&bank).await? {
            None => {
                return Ok(Some(Err(MarketFailure::NoBankAccount)));
            }
            Some(account) if account.money < cost => {
                return Ok(Some(Err(MarketFailure::InsufficientMoney(cost as i64))));
            }
            Some(_) => {}
        }

        let prior = self
            .offers
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

    pub async fn sell_coin(
        &self,
        coin: &str,
        amount: f64,
        price: f64,
        seller: WalletId,
        subwallet: &str,
    ) -> Result<Option<Result<Trade, MarketFailure>>> {
        let Some(coin_id) = self.coins.coin_id(coin).await? else {
            return Ok(Some(Err(MarketFailure::UnknownCoin(coin.to_owned()))));
        };

        if let Err(refusal) = check_order(amount, price) {
            return Ok(Some(Err(MarketFailure::Refusal(refusal))));
        }

        let Some(wallet) = self.wallets.wallet(seller).await? else {
            return Ok(None);
        };
        let (user, bank) = (wallet.user, wallet.bank);
        let Some(account) = self.subwallets.subwallet(coin_id, subwallet).await? else {
            return Ok(Some(Err(MarketFailure::MissingSubwallet(
                subwallet.to_owned(),
            ))));
        };

        if account.balance < amount {
            return Ok(Some(Err(MarketFailure::InsufficientCoins)));
        }

        if self.banks.bank(&bank).await?.is_none() {
            return Ok(Some(Err(MarketFailure::NoBankAccount)));
        }

        if let Some(offer) = self.offers.offer_of(coin_id, seller).await? {
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

    pub async fn cancel_trade(&self, coin: &str, wallet: WalletId) -> Result<bool> {
        let Some(id) = self.coins.coin_id(coin).await? else {
            return Ok(false);
        };

        if self.trader(wallet).await?.is_none() {
            return Ok(false);
        }

        self.offers.remove_offer(id, wallet).await
    }
}

impl Market {
    pub async fn trader(&self, id: WalletId) -> Result<Option<(String, String)>> {
        Ok(self
            .wallets
            .wallet(id)
            .await?
            .map(|wallet| (wallet.user, wallet.bank)))
    }

    pub(crate) async fn trade(
        &self,
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
            let offers = self.offers.offers(coin_id, side.other()).await?;
            let Some(mut offer) = offers
                .into_iter()
                .find(|offer| side.accepts(offer.price, price) && offer.amount > 0.0)
            else {
                break;
            };
            let Some(other) = self.wallets.wallet(offer.wallet).await? else {
                self.offers.remove_offer(coin_id, offer.wallet).await?;
                continue;
            };
            let trade_amount = unmatched_amount.min(offer.amount);
            let offer_account = self.subwallets.subwallet(coin_id, &offer.subwallet).await?;
            // As in `BlockchainSystem.SellCoin`/`BuyCoin`, offers are invalid if their sub-wallet
            // is missing, owner cannot pay, or balance is below the offered amount.
            let affordable = match (side, &offer_account) {
                (_, None) => false,
                (Side::Sell, Some(_)) => self
                    .banks
                    .bank(&other.bank)
                    .await?
                    .is_some_and(|bank| bank.money >= trade_amount * offer.price),
                (Side::Buy, Some(account)) => account.balance >= offer.amount,
            };

            if !affordable {
                self.offers.remove_offer(coin_id, offer.wallet).await?;
                continue;
            }

            let ((buyer, buyer_bank), (seller, seller_bank)) = match side {
                Side::Buy => ((trader, &bank), (other.id, &other.bank)),
                Side::Sell => ((other.id, &other.bank), (trader, &bank)),
            };

            self.bank
                .transfer_money(buyer_bank, seller_bank, trade_amount * offer.price)
                .await?;
            self.wallets
                .add_balance(seller, coin, -trade_amount)
                .await?;
            self.wallets.add_balance(buyer, coin, trade_amount).await?;

            let delta = if side == Side::Buy {
                trade_amount
            } else {
                -trade_amount
            };

            if let Some(mut account) = self.subwallets.subwallet(coin_id, subwallet).await? {
                account.balance = clamp_balance(account.balance + delta);
                self.subwallets.save_subwallet(&account).await?;
            }

            // The offer's sub-wallet takes the opposite side (`BlockchainSystem.AutoTransaction`).
            if let Some(mut account) = offer_account {
                account.balance = clamp_balance(account.balance - delta);
                self.subwallets.save_subwallet(&account).await?;
            }

            // Every match contributes to the next daily coin price
            // (`BlockchainSystem.AutoTransaction`).
            self.record_trade(coin_id, offer.price as i64, trade_amount as i64)
                .await?;
            unmatched_amount -= trade_amount;
            matched_amount += trade_amount;

            if offer.amount - trade_amount <= 0.0 {
                self.offers.remove_offer(coin_id, offer.wallet).await?;
            } else {
                offer.amount -= trade_amount;
                self.offers.save_offer(&offer).await?;
            }

            if unmatched_amount <= 0.0 {
                return Ok(Trade::Complete);
            }
        }

        self.offers
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
pub(crate) struct Order<'a> {
    amount: f64,
    price: f64,
    subwallet: &'a str,
}

/// A world service with only its declared dependencies.
pub struct Market {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) banks: std::rc::Rc<crate::storage::BankRepository>,
    pub(crate) coin_rows: std::rc::Rc<crate::storage::CoinRepository>,
    pub(crate) wallet_rows: std::rc::Rc<crate::storage::WalletRepository>,
    pub(crate) offers: std::rc::Rc<crate::storage::OfferRepository>,
    pub(crate) subwallets: std::rc::Rc<crate::storage::SubwalletRepository>,
    pub(crate) bank: greyhack_world_modules::Dep<crate::services::Bank>,
    pub(crate) coins: greyhack_world_modules::Dep<crate::services::CoinService>,
    pub(crate) wallets: greyhack_world_modules::Dep<crate::services::WalletService>,
}

impl Market {
    pub(crate) fn new(
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
        banks: std::rc::Rc<crate::storage::BankRepository>,
        coin_rows: std::rc::Rc<crate::storage::CoinRepository>,
        wallet_rows: std::rc::Rc<crate::storage::WalletRepository>,
        offers: std::rc::Rc<crate::storage::OfferRepository>,
        subwallets: std::rc::Rc<crate::storage::SubwalletRepository>,
    ) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            banks,
            coin_rows,
            wallet_rows,
            offers,
            subwallets,
            bank: registry.get(),
            coins: registry.get(),
            wallets: registry.get(),
        }
    }
}
