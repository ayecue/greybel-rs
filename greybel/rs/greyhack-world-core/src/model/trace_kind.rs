//! What ended the player's connection.

/// What traced the player when they got a strike (`PlayerUtils.Trace`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceKind {
    /// An active trace ran out.
    Active,
    /// A passive trace followed the connection back to the player.
    Passive,
}
