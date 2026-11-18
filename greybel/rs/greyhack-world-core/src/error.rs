/// Invalid network layout.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LayoutFailure {
    /// The parent index does not reference an earlier node.
    #[error("node {index} hangs under node {parent}, which isn't before it")]
    ParentOrder { index: usize, parent: usize },
    /// A hub is missing its parent device.
    #[error("hub {index} hangs under no device")]
    HubWithoutParent { index: usize },
    /// A computer is missing its parent device or main router.
    #[error("computer {index} hangs under no device")]
    ComputerWithoutParent { index: usize },
    /// The first node must be a router.
    #[error("the first node isn't a router")]
    FirstNodeNotRouter,
    /// The layout contains no player computer.
    #[error("no computer is the player's")]
    NoPlayerComputer,
}

/// A world operation failed because a database read or write failed.
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    /// An unhandled game exception stops the running script.
    #[error("the game crashed")]
    GameCrash,
    #[error(transparent)]
    Database(#[from] turso::Error),
    /// A scheduled domain routine failed.
    #[error(transparent)]
    Routine(#[from] greyhack_world_effects::RoutineError),
    /// An asynchronous event subscriber failed.
    #[error(transparent)]
    Event(#[from] greyhack_world_effects::EventError),
    /// Nonempty library content is not valid JSON (`PlayerIntrinsics.include_lib`).
    #[error("malformed library JSON: {0}")]
    MalformedLibrary(String),
    /// Persisted auxiliary state is not valid JSON.
    #[error("invalid saved state: {0}")]
    InvalidState(String),
    /// A `NetworkLayout` references a missing or later device, or gives a computer no
    /// parent.
    #[error("invalid network layout: {0}")]
    Layout(LayoutFailure),
}

pub type Result<T, E = WorldError> = std::result::Result<T, E>;
