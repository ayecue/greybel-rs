use crate::protocol::ScriptFailure;

/// Failure that prevents a route from returning a program result.
#[derive(Debug, thiserror::Error)]
pub enum RouteError<E> {
    /// Script failure raised with game-compatible behavior.
    #[error("{0:?}")]
    Script(ScriptFailure),
    /// Failure of the world service called by the route.
    #[error(transparent)]
    World(E),
    /// Object ID is absent from the program tree or has the wrong class.
    #[error("unknown object")]
    UnknownObject,
}

impl<E> From<E> for RouteError<E> {
    fn from(error: E) -> Self {
        Self::World(error)
    }
}

/// Result returned by a route.
pub type RouteResult<T, E> = std::result::Result<T, RouteError<E>>;
