use greybel_interpreter::{NativeError, RuntimeErrorKind};

use crate::protocol::{Failure, ScriptFailure};

/// Errors raised by Grey Hack base intrinsics, using the game's messages.
/// Hosts can downcast them with `RuntimeError::native::<IntrinsicError>()`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntrinsicError {
    /// An unmodified standard MiniScript intrinsic failed.
    #[error(transparent)]
    MiniScript(#[from] greybel_ms_intrinsics::IntrinsicError),
    #[error("char: invalid char code.")]
    CharInvalid,
    #[error("code: empty string")]
    CodeEmpty,
    #[error("code: invalid char code.")]
    CodeInvalid,
    #[error("join: string too large")]
    JoinTooLarge,
    #[error("join: delimiter too large")]
    JoinDelimiterTooLarge,
    #[error("push: Unable to stack to itself")]
    PushSelf,
    /// A collection method received a game object.
    #[error("{0}: This object does not allow the use of this method")]
    GameObject(&'static str),
    #[error("push: value cannot be null.")]
    PushNull,
    #[error("range() \"from\" and \"to\" parameters cannot be null")]
    RangeNull,
    /// A string `replace` call used function syntax instead of method syntax.
    #[error(
        "invalid replace invocation: replace must be called from the string itself, for example: {0}.replace"
    )]
    ReplaceOnString(String),
    #[error("wait: time must have a value between 0.01 and 300")]
    WaitOutOfRange,
    /// An intrinsic received missing or incorrectly typed arguments.
    #[error("{0}: Invalid arguments")]
    InvalidArguments(&'static str),
    /// A game intrinsic ran without [`Game`](crate::Game) host data.
    #[error("the game world is not set up")]
    NoGame,
    /// The game server connection closed or failed to exchange an operation message.
    #[error("the game server is unreachable: {0}")]
    GameServer(duplex_core::Error<ScriptFailure>),
    /// The game server returned a failure impossible for the operation.
    #[error("the game server failed unexpectedly: {0:?}")]
    UnexpectedFailure(ScriptFailure),
    /// The game server returned a refusal without intrinsic-specific text.
    #[error("{intrinsic}: refused with a failure it can't return: {failure:?}")]
    UnreturnedRefusal {
        intrinsic: &'static str,
        failure: Failure,
    },
    /// No runner is installed for server-side SSH encode/decode scripts.
    #[error("the SSH script runner is unavailable")]
    NoChildRunner,
    /// A game failure terminates the script with Grey Hack's script error.
    #[error("{}", RuntimeErrorKind::GameCrash)]
    GameCrash,
    #[error("get_abs_path: path too large")]
    PathTooLarge,
    #[error("get_abs_path: basePath too large")]
    BasePathTooLarge,
    #[error("You must wait 0.1 seconds before you can launch clear_screen again")]
    ClearScreenTooSoon,
    /// An intrinsic-specific name-length limit was exceeded.
    #[error("{0} cannot exceed the 15 character limit.")]
    TooLong(&'static str),
    /// `copy`, `move`, or `symlink` received a new name longer than 128 characters.
    #[error("{0}: name cannot exceed the 128 character limit.")]
    NameTooLong(&'static str),
    /// A path contains more than 16 folders.
    #[error("{0}: path too large")]
    TooManyFolders(&'static str),
    #[error("Only alphanumeric characters are allowed for owner names.")]
    InvalidOwner,
    #[error("Owner cannot exceed the 15 character limit.")]
    OwnerTooLong,
    #[error("Only alphanumeric characters are allowed for group names.")]
    InvalidGroup,
    #[error("Group name cannot exceed the 15 character limit.")]
    GroupTooLong,
    /// Grey Hack raised an error with custom text.
    #[error("{0}")]
    Message(&'static str),
    /// A network card is disabled; Grey Hack reports `wifi_networks` for every intrinsic.
    #[error("wifi_networks: wifi card is disabled")]
    WifiCardInactive,
    /// A server-side SSH encode/decode script called a forbidden intrinsic.
    #[error("{0}: unable to use this method in encryption configuration")]
    LoginEncryption(&'static str),
    /// The player's required network is down.
    #[error("{0}: No internet connection")]
    NoConnection(&'static str),
    /// The host computer has no network required by the intrinsic.
    #[error("{0}: No internet access.")]
    NoAccess(&'static str),
    /// A regex option is not `none`, `i`, `m`, `s`, `n`, or `x`.
    #[error("{0}: Invalid regex option")]
    InvalidRegexOption(&'static str),
    /// Regex matching exceeded Grey Hack's 500 ms timeout.
    #[error("{0}: operation timeout")]
    RegexTimeout(&'static str),
    /// The regex engine reported a failure.
    #[error("{0}")]
    InvalidPattern(String),
    #[error("Type Error: '{0}' pattern can't be empty or null")]
    EmptyPattern(&'static str),
    #[error("Type Error: '{0}' requires string")]
    RequiresString(&'static str),
}

impl NativeError for IntrinsicError {
    /// Grey Hack returns these error messages instead of raising them for invalid arguments, over-limit names or paths, invalid owners or groups, missing networks, disabled cards, and invalid regexes.
    /// Exceptions are `connect_service`'s missing connection and `is_match`'s invalid regex option.
    fn is_returned(&self) -> bool {
        match self {
            Self::InvalidArguments(_)
            | Self::TooLong(_)
            | Self::NameTooLong(_)
            | Self::TooManyFolders(_)
            | Self::PathTooLarge
            | Self::BasePathTooLarge
            | Self::InvalidOwner
            | Self::OwnerTooLong
            | Self::InvalidGroup
            | Self::GroupTooLong
            | Self::NoAccess(_)
            | Self::WifiCardInactive
            | Self::InvalidPattern(_) => true,
            Self::NoConnection(intrinsic) => *intrinsic != "connect_service",
            Self::InvalidRegexOption(intrinsic) => *intrinsic != "is_match",
            _ => false,
        }
    }
}
