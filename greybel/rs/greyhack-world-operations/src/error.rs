use greyhack_world_core::WorldError;
pub use greyhack_world_router::RouteError;

/// Failure that prevents an operation from returning a program result.
pub type OperationError = RouteError<WorldError>;

/// Result of a world operation.
pub type Result<T, E = OperationError> = std::result::Result<T, E>;
