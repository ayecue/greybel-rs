//! Cycle operations available to wait routes.

use std::any::Any;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use greyhack_world_programs::protocol::{ObjectId, ProgramId};
use greyhack_world_programs::{Process, ProgramKind, RouteResult, TreeId};

use crate::{Call, RouteContext, WorldContext};

/// Future returned by an erased cycle operation.
pub type CycleFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A typed wait-route cycle packaged as a type-erased result.
#[doc(hidden)]
pub trait ErasedCycleCall<W: WorldContext, E> {
    /// Runs the waiting route's cycle.
    fn run<'a>(
        self: Box<Self>,
        context: &'a RouteContext<'a, W>,
        call: Call,
    ) -> CycleFuture<'a, RouteResult<Box<dyn Any>, E>>
    where
        Self: 'a;
}

struct FnCycleCall<F>(RefCell<Option<F>>);

impl<F, T, W, E> ErasedCycleCall<W, E> for FnCycleCall<F>
where
    F: AsyncFnOnce(&RouteContext<'_, W>, Call) -> RouteResult<T, E>,
    T: 'static,
    W: WorldContext,
{
    fn run<'a>(
        self: Box<Self>,
        context: &'a RouteContext<'a, W>,
        call: Call,
    ) -> CycleFuture<'a, RouteResult<Box<dyn Any>, E>>
    where
        F: 'a,
    {
        let call_fn = self
            .0
            .borrow_mut()
            .take()
            .expect("a cycle closure runs once");

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

struct FnWorldCall<F>(RefCell<Option<F>>);

impl<F, T, W> ErasedWorldCall<W> for FnWorldCall<F>
where
    F: AsyncFnOnce(&W) -> T,
    T: 'static,
{
    fn run<'a>(self: Box<Self>, world: &'a W) -> CycleFuture<'a, Box<dyn Any>>
    where
        F: 'a,
    {
        let call = self
            .0
            .borrow_mut()
            .take()
            .expect("a world closure runs once");

        Box::pin(async move { Box::new(call(world).await) as Box<dyn Any> })
    }
}

/// Server implementation used by [`CycleAccess`] to run wait-route operations.
pub trait CycleDriver<W: WorldContext, E>: Any {
    /// Runs a route cycle, optionally drawing its operation budget.
    fn change<'a>(
        &'a self,
        connection: u32,
        program: ProgramId,
        budgeted: bool,
        apply: Box<dyn ErasedCycleCall<W, E> + 'a>,
    ) -> CycleFuture<'a, RouteResult<Box<dyn Any>, E>>;

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
    ) -> RouteResult<ProgramId, E>;

    /// Waits for typed work, returning `None` on cancellation.
    fn defer<'a>(
        &'a self,
        connection: u32,
        program: ProgramId,
        work: u64,
    ) -> CycleFuture<'a, Option<Box<dyn Any>>>;

    /// Returns the per-tick routine limit.
    fn routine_budget(&self) -> usize;

    /// Flushes pending world changes.
    fn save<'a>(&'a self) -> CycleFuture<'a, bool>;
}

/// Sized shared reference used to recover a typed cycle driver from [`CycleAccess`].
#[doc(hidden)]
pub struct CycleDriverRef<W: WorldContext, E>(Rc<dyn CycleDriver<W, E>>);

impl<W: WorldContext, E> CycleDriverRef<W, E> {
    /// Wraps a shared cycle driver.
    pub fn new<D: CycleDriver<W, E> + 'static>(driver: Rc<D>) -> Self {
        Self(driver)
    }
}

/// World-cycle access supplied to a wait route.
#[derive(Clone, Copy)]
pub struct CycleAccess<'a> {
    state: &'a dyn Any,
    driver: &'a dyn Any,
    connection: u32,
}

impl<'a> CycleAccess<'a> {
    /// Creates cycle access for a server state and its driver.
    pub fn new<T: 'static, W: WorldContext + 'static, E: 'static>(
        state: &'a T,
        connection: u32,
        driver: &'a CycleDriverRef<W, E>,
    ) -> Self {
        Self {
            state,
            driver,
            connection,
        }
    }

    /// Returns the concrete server state.
    #[must_use]
    pub fn state<T: 'static>(&self) -> &'a T {
        self.state
            .downcast_ref::<T>()
            .expect("routes receive cycles of their own server")
    }

    /// Returns the runtime connection whose program is waiting.
    #[must_use]
    pub const fn connection(&self) -> u32 {
        self.connection
    }

    fn driver<W: WorldContext + 'static, E: 'static>(&self) -> &'a dyn CycleDriver<W, E> {
        self.driver
            .downcast_ref::<CycleDriverRef<W, E>>()
            .expect("cycle driver matches its world")
            .0
            .as_ref()
    }

    /// Runs a budgeted or unbudgeted cycle with a typed result.
    pub async fn change_dynamic<T: 'static, W: WorldContext + 'static, E: 'static>(
        self,
        program: ProgramId,
        budgeted: bool,
        apply: impl AsyncFnOnce(&RouteContext<'_, W>, Call) -> RouteResult<T, E>,
    ) -> RouteResult<T, E> {
        let apply = Box::new(FnCycleCall(RefCell::new(Some(apply))));

        self.driver::<W, E>()
            .change(self.connection, program, budgeted, apply)
            .await
            .map(|value| {
                *value
                    .downcast::<T>()
                    .expect("cycle result has its declared type")
            })
    }

    /// Runs an unbudgeted world-only cycle.
    pub async fn world_dynamic<T: 'static, W: WorldContext + 'static, E: 'static>(
        self,
        apply: impl AsyncFnOnce(&W) -> T,
    ) -> T {
        let apply = Box::new(FnWorldCall(RefCell::new(Some(apply))));

        *self
            .driver::<W, E>()
            .world(apply)
            .await
            .downcast::<T>()
            .expect("world result has its declared type")
    }

    /// Allocates and pushes a child program.
    pub fn push_dynamic<W: WorldContext + 'static, E: 'static>(
        self,
        tree: TreeId,
        process: Option<Process<W::Computer>>,
        session: ObjectId,
        parent: ProgramId,
        kind: ProgramKind<W::Computer>,
    ) -> RouteResult<ProgramId, E> {
        self.driver::<W, E>()
            .push(tree, process, session, parent, kind)
    }

    /// Waits for a typed result, returning `None` when cancelled.
    pub async fn defer_dynamic<T: 'static, W: WorldContext + 'static, E: 'static>(
        self,
        program: ProgramId,
        work: u64,
    ) -> Option<T> {
        self.driver::<W, E>()
            .defer(self.connection, program, work)
            .await
            .map(|value| {
                *value
                    .downcast::<T>()
                    .expect("work result has its declared type")
            })
    }

    /// Returns the per-tick routine limit.
    pub fn routine_budget_dynamic<W: WorldContext + 'static, E: 'static>(self) -> usize {
        self.driver::<W, E>().routine_budget()
    }

    /// Flushes pending world changes.
    pub async fn save_dynamic<W: WorldContext + 'static, E: 'static>(self) -> bool {
        self.driver::<W, E>().save().await
    }
}
