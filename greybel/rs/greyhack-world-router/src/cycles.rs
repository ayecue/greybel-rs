//! Cycle operations for wait routes. A cycle is one serialized world step; wait routes use these
//! operations when they resume after awaiting work or advancing world time.

use std::any::Any;
use std::future::Future;
use std::pin::Pin;

use greyhack_world_effects::WorkId;
use greyhack_world_programs::protocol::{ObjectId, ProgramId};
use greyhack_world_programs::{Process, ProgramKind, RouteResult, TreeId};

use crate::{Call, RouteContext, WorldContext};

/// Future returned by an erased cycle operation.
pub type CycleFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A typed wait-route cycle packaged as a type-erased result.
#[doc(hidden)]
pub trait ErasedCycleCall<W: WorldContext> {
    /// Runs the waiting route's cycle.
    fn run<'a>(
        self: Box<Self>,
        context: &'a RouteContext<'a, W>,
        call: Call,
    ) -> CycleFuture<'a, RouteResult<Box<dyn Any>, W::Error>>
    where
        Self: 'a;
}

struct FnCycleCall<F>(F);

impl<F, T, W> ErasedCycleCall<W> for FnCycleCall<F>
where
    F: AsyncFnOnce(&RouteContext<'_, W>, Call) -> RouteResult<T, W::Error>,
    T: 'static,
    W: WorldContext,
{
    fn run<'a>(
        self: Box<Self>,
        context: &'a RouteContext<'a, W>,
        call: Call,
    ) -> CycleFuture<'a, RouteResult<Box<dyn Any>, W::Error>>
    where
        F: 'a,
    {
        let call_fn = self.0;

        Box::pin(async move {
            call_fn(context, call)
                .await
                .map(|value| Box::new(value) as Box<dyn Any>)
        })
    }
}

/// A typed world-only cycle packaged as a type-erased result.
#[doc(hidden)]
pub trait ErasedWorldCall<W> {
    /// Runs the world-only closure.
    fn run<'a>(self: Box<Self>, world: &'a W) -> CycleFuture<'a, Box<dyn Any>>
    where
        Self: 'a;
}

struct FnWorldCall<F>(F);

impl<F, T, W> ErasedWorldCall<W> for FnWorldCall<F>
where
    F: AsyncFnOnce(&W) -> T,
    T: 'static,
{
    fn run<'a>(self: Box<Self>, world: &'a W) -> CycleFuture<'a, Box<dyn Any>>
    where
        F: 'a,
    {
        let call = self.0;

        Box::pin(async move { Box::new(call(world).await) as Box<dyn Any> })
    }
}

/// Server operations that execute cycles on behalf of a wait route.
pub trait CycleDriver<W: WorldContext> {
    /// Runs a route cycle, optionally drawing its operation budget.
    fn change<'a>(
        &'a self,
        connection: u32,
        program: ProgramId,
        budgeted: bool,
        apply: Box<dyn ErasedCycleCall<W> + 'a>,
    ) -> CycleFuture<'a, RouteResult<Box<dyn Any>, W::Error>>;

    /// Runs an unbudgeted world-only cycle.
    fn world<'a>(
        &'a self,
        apply: Box<dyn ErasedWorldCall<W> + 'a>,
    ) -> CycleFuture<'a, Box<dyn Any>>;

    /// Allocates and pushes a child program.
    fn push(
        &self,
        tree: TreeId,
        process: Option<Process<W::Computer>>,
        session: ObjectId,
        parent: ProgramId,
        kind: ProgramKind<W::Computer>,
    ) -> RouteResult<ProgramId, W::Error>;

    /// Waits for typed work, returning `None` on cancellation.
    fn defer<'a>(
        &'a self,
        connection: u32,
        program: ProgramId,
        work: WorkId,
    ) -> CycleFuture<'a, Option<Box<dyn Any>>>;

    /// Returns the per-tick routine limit.
    fn routine_budget(&self) -> usize;

    /// Flushes pending world changes.
    fn save<'a>(&'a self) -> CycleFuture<'a, bool>;
}

/// Typed cycle operations supplied to a wait route.
pub struct CycleAccess<'a, W: WorldContext> {
    driver: &'a dyn CycleDriver<W>,
    connection: u32,
}

impl<W: WorldContext> Copy for CycleAccess<'_, W> {}

impl<W: WorldContext> Clone for CycleAccess<'_, W> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, W: WorldContext> CycleAccess<'a, W> {
    /// Creates cycle access for a connection and its driver.
    pub const fn new(connection: u32, driver: &'a dyn CycleDriver<W>) -> Self {
        Self { driver, connection }
    }

    /// Returns the runtime connection whose program is waiting.
    #[must_use]
    pub const fn connection(&self) -> u32 {
        self.connection
    }

    /// Runs a budgeted or unbudgeted cycle with a typed result.
    pub async fn change<T: 'static>(
        self,
        program: ProgramId,
        budgeted: bool,
        apply: impl AsyncFnOnce(&RouteContext<'_, W>, Call) -> RouteResult<T, W::Error>,
    ) -> RouteResult<T, W::Error> {
        let apply = Box::new(FnCycleCall(apply));

        self.driver
            .change(self.connection, program, budgeted, apply)
            .await
            .map(|value| {
                *value
                    .downcast::<T>()
                    .expect("cycle result has its declared type")
            })
    }

    /// Runs an unbudgeted world-only cycle.
    pub async fn world<T: 'static>(self, apply: impl AsyncFnOnce(&W) -> T) -> T {
        let apply = Box::new(FnWorldCall(apply));

        *self
            .driver
            .world(apply)
            .await
            .downcast::<T>()
            .expect("world result has its declared type")
    }

    /// Allocates and pushes a child program.
    pub fn push(
        self,
        tree: TreeId,
        process: Option<Process<W::Computer>>,
        session: ObjectId,
        parent: ProgramId,
        kind: ProgramKind<W::Computer>,
    ) -> RouteResult<ProgramId, W::Error> {
        self.driver.push(tree, process, session, parent, kind)
    }

    /// Waits for a typed result, returning `None` when cancelled.
    pub async fn defer<T: 'static>(self, program: ProgramId, work: WorkId) -> Option<T> {
        self.driver
            .defer(self.connection, program, work)
            .await
            .map(|value| {
                *value
                    .downcast::<T>()
                    .expect("work result has its declared type")
            })
    }

    /// Returns the per-tick routine limit.
    pub fn routine_budget(self) -> usize {
        self.driver.routine_budget()
    }

    /// Flushes pending world changes.
    pub async fn save(self) -> bool {
        self.driver.save().await
    }
}
