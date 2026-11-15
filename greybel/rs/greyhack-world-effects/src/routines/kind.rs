//! The kinds of routines a scheduler runs.

use std::any::Any;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use greyhack_world_storage::TableSet;
use turso::Connection;

/// Identifier of a routine queue row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoutineId(u64);

impl RoutineId {
    /// Creates a routine identifier from its numeric representation.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// Returns the numeric representation used by routine storage.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A boxed future that is not required to be `Send`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Error returned by a routine instance.
pub struct RoutineError(Box<dyn std::error::Error>);

impl RoutineError {
    /// Erases a concrete routine error while retaining its source.
    pub fn new(error: impl std::error::Error + 'static) -> Self {
        Self(Box::new(error))
    }
}

impl Display for RoutineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl std::error::Error for RoutineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

impl From<turso::Error> for RoutineError {
    fn from(error: turso::Error) -> Self {
        Self::new(error)
    }
}

/// Result returned by routine storage and execution.
pub type Result<T> = std::result::Result<T, RoutineError>;

/// One kind of routine, built with the services and repositories it calls.
pub trait Routine: 'static {
    /// Data stored for one queue row.
    type Data: Any;

    /// Stable string key stored in `routines.kind`.
    const KEY: &'static str;
    /// Order among routines that are equally late; lower runs first.
    const PRIORITY: i64;

    /// Returns the tables that store this kind's data.
    fn tables() -> TableSet;

    /// Stores `data` for queue row `id`.
    fn store(
        &self,
        conn: &Connection,
        id: RoutineId,
        data: &Self::Data,
    ) -> impl Future<Output = Result<()>>;

    /// Loads the data of queue row `id`, or `None` after its owner was removed.
    fn load(
        &self,
        conn: &Connection,
        id: RoutineId,
    ) -> impl Future<Output = Result<Option<Self::Data>>>;

    /// Runs a due routine after its queue row has been removed.
    fn run(&self, id: RoutineId, data: Self::Data) -> impl Future<Output = Result<()>>;

    /// Notifies a waiter when its queue row is removed without running.
    fn on_remove(&self, _id: RoutineId) -> impl Future<Output = Result<()>> {
        async { Ok(()) }
    }

    /// Cancels this kind's routines that refer to removed computer IDs.
    fn cancel_for_computers(
        &self,
        _conn: &Connection,
        _computers: &[u32],
    ) -> impl Future<Output = Result<()>> {
        async { Ok(()) }
    }
}

/// A routine kind with its data type hidden behind [`Any`].
pub(crate) trait DynRoutine {
    fn key(&self) -> &'static str;
    fn priority(&self) -> i64;
    fn tables(&self) -> TableSet;
    fn store<'a>(
        &'a self,
        conn: &'a Connection,
        id: RoutineId,
        data: &'a dyn Any,
    ) -> BoxFuture<'a, Result<()>>;

    fn load<'a>(
        &'a self,
        conn: &'a Connection,
        id: RoutineId,
    ) -> BoxFuture<'a, Result<Option<Box<dyn Any>>>>;

    fn run<'a>(&'a self, id: RoutineId, data: Box<dyn Any>) -> BoxFuture<'a, Result<()>>;
    fn on_remove<'a>(&'a self, id: RoutineId) -> BoxFuture<'a, Result<()>>;
    fn cancel_for_computers<'a>(
        &'a self,
        conn: &'a Connection,
        computers: &'a [u32],
    ) -> BoxFuture<'a, Result<()>>;
}

/// Adapts routine `R` to [`DynRoutine`].
pub(crate) struct Adapter<R>(pub(crate) Rc<R>);

impl<R: Routine> DynRoutine for Adapter<R> {
    fn key(&self) -> &'static str {
        R::KEY
    }

    fn priority(&self) -> i64 {
        R::PRIORITY
    }

    fn tables(&self) -> TableSet {
        R::tables()
    }

    fn store<'a>(
        &'a self,
        conn: &'a Connection,
        id: RoutineId,
        data: &'a dyn Any,
    ) -> BoxFuture<'a, Result<()>> {
        let data = data
            .downcast_ref::<R::Data>()
            .expect("a scheduled routine has the data of its kind");

        Box::pin(self.0.store(conn, id, data))
    }

    fn load<'a>(
        &'a self,
        conn: &'a Connection,
        id: RoutineId,
    ) -> BoxFuture<'a, Result<Option<Box<dyn Any>>>> {
        Box::pin(async move {
            Ok(self
                .0
                .load(conn, id)
                .await?
                .map(|data| Box::new(data) as Box<dyn Any>))
        })
    }

    fn run<'a>(&'a self, id: RoutineId, data: Box<dyn Any>) -> BoxFuture<'a, Result<()>> {
        let data = *data
            .downcast::<R::Data>()
            .expect("a queue row has the data of its kind");

        Box::pin(self.0.run(id, data))
    }

    fn on_remove<'a>(&'a self, id: RoutineId) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.0.on_remove(id))
    }

    fn cancel_for_computers<'a>(
        &'a self,
        conn: &'a Connection,
        computers: &'a [u32],
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(self.0.cancel_for_computers(conn, computers))
    }
}

impl Debug for RoutineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, formatter)
    }
}
