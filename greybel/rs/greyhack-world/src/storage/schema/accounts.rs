//! Passwords, bank accounts and subscriptions, mail accounts, and persons.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "
-- every hash the world ever set, as the game's `Passwords` table; never deleted
CREATE TABLE IF NOT EXISTS passwords (
    hash  TEXT PRIMARY KEY,
    plain TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS bank_accounts (
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

-- `npc_contents` is `UserMail.npcContentsGenerated`: whether an NPC account's mails were drawn, which happens once
CREATE TABLE IF NOT EXISTS mail_accounts (
    address       TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL REFERENCES passwords(hash),
    npc_contents  INTEGER NOT NULL DEFAULT 0
);

-- who a user account belongs to (`Persona`)
CREATE TABLE IF NOT EXISTS persons (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL,
    surname      TEXT NOT NULL,
    gender       INTEGER NOT NULL,
    mail_address TEXT REFERENCES mail_accounts(address) ON DELETE SET NULL,
    bank_number  TEXT REFERENCES bank_accounts(number) ON DELETE SET NULL
);

",
    tables: &[
    "passwords",
    "bank_accounts",
    "bank_transactions",
    "bank_subscriptions",
    "bank_subscription_networks",
    "mail_accounts",
    "persons",
    ],
};
