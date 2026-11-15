use duplex_core::{Exchange, Message};
use greyhack_world_programs::protocol::ScriptFailure;

/// Operation result `R` or raised script failure.
pub type OperationResponse<R> = duplex_core::ValueResponse<R, ScriptFailure>;
/// Responder associated with message `M`.
pub type ResponderOf<M> = <<M as Message>::Exchange as Exchange>::Responder;
