/// A conversation put into an NPC's mailbox (`ItemsWorldGen.AddNpcConvMail`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConversationMail {
    /// Account name of the NPC.
    pub user: String,
    /// Game text key of the conversation lines.
    pub conversation: String,
    /// Address of the other party (`HelperConversation.users[RECEIVE]`).
    pub receive_user: String,
}
