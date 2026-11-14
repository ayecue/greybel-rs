/// A mail put into a mailbox (`UserMail.RecibirMail`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Delivery {
    /// Mailbox address.
    pub to: String,
    /// Other party (`Mail.otherMail`).
    pub other: String,
    pub subject: String,
    pub text: String,
    /// Whether the mail was sent by the mailbox owner (`isSending`): its text starts with `> `.
    pub sending: bool,
}
