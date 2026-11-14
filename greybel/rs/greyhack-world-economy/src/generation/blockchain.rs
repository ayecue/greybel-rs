//! Seeded generation of default `BlockchainSystem` coins.

use greyhack_behaviour::hash_password;

use crate::{Coin, PriceRecord, Rng, Word};

const DEFAULT_COINS: &[&str] = &["Ether", "Bitcoin", "PennyBucks", "GridCoin", "AnonCash"];

/// Coin generation draft with creator password and price history.
pub struct CoinDraft {
    /// Generated coin entity.
    pub coin: Coin,
    /// Creator's plaintext password for the core password registry.
    pub password: String,
    /// Generated initial price history.
    pub prices: Vec<PriceRecord>,
}

/// Builds default coins with generated prices, mined balances, and price history dated `date`.
pub fn default_coins(rng: &mut Rng, date: &str) -> Vec<CoinDraft> {
    DEFAULT_COINS
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let round2 = |value: f64| (value * 100.0).round() / 100.0;
            let price = f64::from(rng.range(1, 1000));

            let creator_user = rng
                .pick(&["satoshi", "vitalik", "finney", "szabo"])
                .to_string();
            let password = Word::Password.generate(rng);
            let coin = Coin {
                id: crate::CoinId(index as u32),
                name: (*name).to_owned(),
                creator_user,
                creator_password_hash: hash_password(&password),
                address: rng.hex(12),
                cycle_hours: f64::from(rng.range(1, 24)),
                last_cycle_at: None,
                reward: 1.0,
                mined_coins: f64::from(rng.range(1000, 100_000)),
                max_supply: 100_000_000.0,
                price,
            };
            let prices = [round2(price * 0.9), round2(price * 0.95), price]
                .into_iter()
                .map(|price| PriceRecord {
                    price,
                    date: date.to_owned(),
                })
                .collect();

            CoinDraft {
                coin,
                password,
                prices,
            }
        })
        .collect()
}
