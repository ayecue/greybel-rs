//! Mail accounts, emails, and their replies.

use greyhack_world_storage::TableSet;

pub(crate) const GROUP: TableSet = TableSet::new(
    "-- `npc_contents` is `UserMail.npcContentsGenerated`: whether an NPC account's mails were drawn, which happens once
CREATE TABLE IF NOT EXISTS mail_accounts (
    address       TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL REFERENCES passwords(hash),
    npc_contents  INTEGER NOT NULL DEFAULT 0
);

-- one optional mail account per person (`Persona.email`), owned by NPC mail
CREATE TABLE IF NOT EXISTS person_mail (
    person_id INTEGER PRIMARY KEY REFERENCES persons(id) ON DELETE CASCADE,
    address   TEXT NOT NULL REFERENCES mail_accounts(address) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS emails (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL REFERENCES mail_accounts(address) ON DELETE CASCADE,
    sender  TEXT NOT NULL,
    subject TEXT NOT NULL,
    body    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS emails_inbox ON emails (address, id);

-- the messages after the first of a mail thread (`UserMail.Mail.messages`), in the order added
CREATE TABLE IF NOT EXISTS email_replies (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id INTEGER NOT NULL REFERENCES emails(id) ON DELETE CASCADE,
    body     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS email_replies_thread ON email_replies (email_id, id);

CREATE TABLE IF NOT EXISTS mail_spam (
    address      TEXT PRIMARY KEY REFERENCES mail_accounts(address) ON DELETE CASCADE,
    player_pc_id TEXT NOT NULL DEFAULT '',
    filter       INTEGER NOT NULL DEFAULT 1,
    blacklist    TEXT NOT NULL DEFAULT '[]'
);

",
    &["mail_accounts", "person_mail", "emails", "email_replies", "mail_spam"],
);
