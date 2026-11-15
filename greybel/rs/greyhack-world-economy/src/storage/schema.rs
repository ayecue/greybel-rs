//! Bank accounts, subscriptions, coins, wallets, sub-wallets, and offers.

use greyhack_world_storage::TableSet;

pub(crate) const GROUP: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS bank_accounts (
    number        TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL REFERENCES passwords(hash),
    money         REAL NOT NULL
);

-- the ten most recent credits, debits and failed monthly payments (`BankAccount.Transaccion`)
CREATE TABLE IF NOT EXISTS bank_transactions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    bank_number TEXT NOT NULL REFERENCES bank_accounts(number) ON DELETE CASCADE,
    account     TEXT NOT NULL,
    amount      INTEGER NOT NULL,
    reason      TEXT NOT NULL,
    date        TEXT NOT NULL,
    success     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS bank_transactions_account ON bank_transactions (bank_number, id);

-- one recurring payment of each kind per account (`BankSubs.Payment`)
CREATE TABLE IF NOT EXISTS bank_subscriptions (
    bank_number     TEXT NOT NULL REFERENCES bank_accounts(number) ON DELETE CASCADE,
    kind            INTEGER NOT NULL,
    amount          INTEGER NOT NULL,
    next_payment_at INTEGER NOT NULL,
    PRIMARY KEY (bank_number, kind)
);

-- the public addresses of the networks a subscription charges for (`BankSubs.Payment.publicIps`), in the order they
-- were added
CREATE TABLE IF NOT EXISTS bank_subscription_networks (
    bank_number TEXT NOT NULL REFERENCES bank_accounts(number) ON DELETE CASCADE,
    kind        INTEGER NOT NULL,
    seq         INTEGER NOT NULL,
    public_ip   TEXT NOT NULL,
    PRIMARY KEY (bank_number, kind, seq)
);

-- one optional bank account per person (`Persona.bankAccount`), owned by economy
CREATE TABLE IF NOT EXISTS person_banks (
    person_id   INTEGER PRIMARY KEY REFERENCES persons(id) ON DELETE CASCADE,
    bank_number TEXT NOT NULL REFERENCES bank_accounts(number) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS coins (
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
    &[
    "bank_accounts",
    "bank_transactions",
    "bank_subscriptions",
    "bank_subscription_networks",
    "person_banks",
    "coins",
    "coin_prices",
    "coin_pending_trades",
    "wallets",
    "wallet_balances",
    "subwallets",
    "subwallet_transactions",
    "offers",
    ],
);

/// The data of coin-round routines.
pub const COIN_ROUNDS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS routine_mining_rounds (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    coin_id    INTEGER NOT NULL UNIQUE REFERENCES coins(id) ON DELETE CASCADE
);

",
    &["routine_mining_rounds"],
);

/// The data of payment routines.
pub const PAYMENTS: TableSet = TableSet::new(
    "-- the timer of a subscription's next payment (`BankSubs.Payment.WaitUntilPayment`); a subscription with no row here is not
-- waiting for its payment
CREATE TABLE IF NOT EXISTS routine_payments (
    routine_id  INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE,
    bank_number TEXT NOT NULL,
    kind        INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS routine_payments_subscription ON routine_payments (bank_number, kind);

",
    &["routine_payments"],
);

/// The data-less marker of daily-upkeep routines.
pub const DAILY_UPKEEP: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS routine_daily_upkeep (
    routine_id INTEGER PRIMARY KEY REFERENCES routines(id) ON DELETE CASCADE
);

",
    &["routine_daily_upkeep"],
);

/// Mining subscribers owned by the economy domain.
pub const MINE_SUBSCRIBERS: TableSet = TableSet::new(
    "-- programs mining a coin through a sub-wallet on a computer (`BlockchainSystem.pcInfo`); volatile at open
CREATE TABLE IF NOT EXISTS mining_subscribers (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    coin_id      INTEGER NOT NULL REFERENCES coins(id) ON DELETE CASCADE,
    subwallet_id INTEGER NOT NULL REFERENCES subwallets(id) ON DELETE CASCADE,
    computer_id  INTEGER NOT NULL REFERENCES computers(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS mining_subscribers_coin ON mining_subscribers (coin_id, id);
CREATE INDEX IF NOT EXISTS mining_subscribers_computer ON mining_subscribers (computer_id);

",
    &["mining_subscribers"],
);

/// Deletes mining subscriptions when the world opens.
pub(crate) const MINE_VOLATILE: TableSet =
    TableSet::volatile("DELETE FROM mining_subscribers;\n");

/// Deletes unfinished mining-round routines when the world opens.
pub(crate) const MINE_ROUND_VOLATILE: TableSet = TableSet::volatile(
    "DELETE FROM routines WHERE id IN (SELECT routine_id FROM routine_mining_rounds);\n",
);
