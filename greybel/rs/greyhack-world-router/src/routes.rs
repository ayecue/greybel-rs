use duplex_core::{Exchange, Message};

use crate::protocol::{GameServer, ScriptFailure};

/// Client peer used to send operation messages.
pub type Client = <GameServer as duplex_core::Peer>::Remote;
/// Operation result `R` or raised script failure.
pub type OperationResponse<R> = duplex_core::ValueResponse<R, ScriptFailure>;
/// Responder associated with message `M`.
pub type ResponderOf<M> = <<M as Message>::Exchange as Exchange>::Responder;
