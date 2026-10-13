//! Emails and their replies.

use super::TableGroup;

pub(super) const GROUP: TableGroup = TableGroup {
    schema: "CREATE TABLE IF NOT EXISTS emails (
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

",
    tables: &["emails", "email_replies"],
};
