/// A conversation a mission puts in the mailbox of the owner of its files (`ItemsWorldGen.AddNpcConvMail`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionMail {
    pub title: String,
    /// The address of the other party (`BuildMissionMailAddress`).
    pub outsider: String,
    /// Whether the outsider writes first (`MailConversItem.useNetworkNpcSender`).
    pub outsider_is_sender: bool,
    /// The messages after tag resolution, each with whether the sender line (0) or the receiver line (1) speaks.
    pub lines: Vec<(u8, String)>,
}
