//! Coins, wallets, sub-wallets, and offers.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "CREATE TABLE IF NOT EXISTS coins (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    name                  TEXT NOT NULL UNIQUE,
    creator_user          TEXT NOT NULL,
    creator_password_hash TEXT NOT NULL REFERENCES passwords(hash),
    address               TEXT NOT NULL,
    mining_cycle          REAL NOT NULL,
    reward                REAL NOT NULL,
    mined_coins           REAL NOT NULL,
    max_supply            REAL NOT NULL,
    price                 REAL NOT NULL,
    last_cycle_at         INTEGER
);
CREATE INDEX IF NOT EXISTS coins_creator ON coins (creator_user, id);

CREATE TABLE IF NOT EXISTS coin_prices (
    id      INTEGER PRIMARY KEY,
    coin_id INTEGER NOT NULL REFERENCES coins(id) ON DELETE CASCADE,
    price   REAL NOT NULL,
    date    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS coin_prices_coin ON coin_prices (coin_id, id);

-- distinct prices waiting for the daily `Coin.UpdateMoneyValue` (`Coin.lastTransactions`)
-- The trades since a coin's last price update: their total value and coin count (`Coin.pendingTransactionsValue`,
-- `pendingTransactionsAmount`).
CREATE TABLE IF NOT EXISTS coin_pending_trades (
    coin_id INTEGER PRIMARY KEY REFERENCES coins(id) ON DELETE CASCADE,
    value   INTEGER NOT NULL,
    amount  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS wallets (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    user              TEXT NOT NULL UNIQUE,
    password_hash     TEXT NOT NULL REFERENCES passwords(hash),
    pin               TEXT NOT NULL,
    bank_number       TEXT NOT NULL REFERENCES bank_accounts(number),
    password_reset_at INTEGER
);

CREATE TABLE IF NOT EXISTS wallet_balances (
    id        INTEGER PRIMARY KEY,
    wallet_id INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    coin_id   INTEGER NOT NULL REFERENCES coins(id) ON DELETE CASCADE,
    amount    REAL NOT NULL,
    UNIQUE (wallet_id, coin_id)
);
CREATE INDEX IF NOT EXISTS wallet_balances_coin ON wallet_balances (coin_id);

CREATE TABLE IF NOT EXISTS subwallets (
    id            INTEGER PRIMARY KEY,
    coin_id       INTEGER NOT NULL REFERENCES coins(id) ON DELETE CASCADE,
    user          TEXT NOT NULL,
    password_hash TEXT NOT NULL REFERENCES passwords(hash),
    balance       REAL NOT NULL,
    info          TEXT NOT NULL,
    wallet_id     INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    UNIQUE (coin_id, user)
);

-- the game keeps one transaction per sub-wallet (`HistTransaction`)
CREATE TABLE IF NOT EXISTS subwallet_transactions (
    subwallet_id INTEGER PRIMARY KEY REFERENCES subwallets(id) ON DELETE CASCADE,
    counterparty TEXT NOT NULL,
    amount       REAL NOT NULL,
    direction    INTEGER NOT NULL,
    date         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS offers (
    id        INTEGER PRIMARY KEY,
    coin_id   INTEGER NOT NULL REFERENCES coins(id) ON DELETE CASCADE,
    wallet_id INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    subwallet TEXT NOT NULL,
    sell      INTEGER NOT NULL,
    price     REAL NOT NULL,
    amount    REAL NOT NULL,
    UNIQUE (coin_id, wallet_id)
);
CREATE INDEX IF NOT EXISTS offers_book ON offers (coin_id, sell, id);

",
    tables: &[
    "coins",
    "coin_prices",
    "coin_pending_trades",
    "wallets",
    "wallet_balances",
    "subwallets",
    "subwallet_transactions",
    "offers",
    ],
};
