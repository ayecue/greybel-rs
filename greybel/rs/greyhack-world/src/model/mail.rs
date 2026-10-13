//! Email accounts and inboxes. Addresses use `user@domain`; the domain must match the account SMTP
//! server. Inboxes list the newest mail first.

/// Inbox mail: a thread of one or more messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email {
    pub id: String,
    pub from: String,
    pub subject: String,
    /// First message of the thread (`UserMail.Mail.messages[0]`).
    pub body: String,
    /// Messages after the first, in the order they were added.
    pub replies: Vec<String>,
}

/// Mail account identified by address and password.
#[derive(Debug, Clone, PartialEq)]
pub struct MailAccount {
    pub address: String,
    pub password_hash: String,
    pub inbox: Vec<Email>,
}

impl MailAccount {}
