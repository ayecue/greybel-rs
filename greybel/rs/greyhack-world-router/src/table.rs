//! Driver-independent route registration.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use duplex_core::{Message, ValidatedDecode, ValidatedEncode};

use crate::operation::Operation;
use crate::protocol::{ProgramId, ScriptFailure};
use crate::{
    Call, Client, OperationResponse, ResponderOf, RouteContext, RouteResult, WorldContext,
};

/// How the server executes a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    /// One cycle whose transaction rolls back.
    Read,
    /// One committed cycle.
    Change,
    /// A route that can wait across cycles.
    Wait,
}

/// Future returned by a type-erased route handler.
pub type RouteFuture<'a, T, E> = Pin<Box<dyn Future<Output = RouteResult<T, E>> + 'a>>;

trait TypedHandler<M, R, W: WorldContext, E> {
    fn call<'a>(
        &'a self,
        context: &'a RouteContext<'a, W>,
        call: Call,
        message: M,
    ) -> RouteFuture<'a, R, E>;
}

struct HandlerFn<F>(F);

impl<M, R, W, E, F> TypedHandler<M, R, W, E> for HandlerFn<F>
where
    M: Message + 'static,
    W: WorldContext,
    F: AsyncFn(&RouteContext<'_, W>, Call, M) -> RouteResult<R, E>,
{
    fn call<'a>(
        &'a self,
        context: &'a RouteContext<'a, W>,
        call: Call,
        message: M,
    ) -> RouteFuture<'a, R, E> {
        Box::pin((self.0)(context, call, message))
    }
}

/// A typed route stored in a [`RouteTable`].
pub struct Route<M: Message, R, W: WorldContext, E> {
    kind: RouteKind,
    handler: Rc<dyn TypedHandler<M, R, W, E>>,
    respond: fn(ResponderOf<M>, R),
    fail: fn(ResponderOf<M>, ScriptFailure),
    program: fn(&M) -> Option<ProgramId>,
}

impl<M: Message, R, W: WorldContext, E> Route<M, R, W, E> {
    /// Returns how the server executes this route.
    #[must_use]
    pub const fn kind(&self) -> RouteKind {
        self.kind
    }

    /// Sends a successful response.
    pub fn respond(&self, responder: ResponderOf<M>, value: R) {
        (self.respond)(responder, value);
    }

    /// Sends a script failure.
    pub fn fail(&self, responder: ResponderOf<M>, failure: ScriptFailure) {
        (self.fail)(responder, failure);
    }

    /// Returns the source program ID.
    pub fn program(&self, message: &M) -> Option<ProgramId> {
        (self.program)(message)
    }

    /// Runs this route with its concrete message and response types.
    pub async fn run(
        &self,
        context: &RouteContext<'_, W>,
        call: Call,
        message: M,
    ) -> RouteResult<R, E> {
        self.handler.call(context, call, message).await
    }
}

/// A typed route packaged for the server's driver-independent cycle dispatcher.
#[doc(hidden)]
pub trait RouteCall<W: WorldContext, E> {
    /// Returns the route's execution kind.
    fn kind(&self) -> RouteKind;
    /// Returns the source program ID.
    fn program(&self) -> Option<ProgramId>;
    /// Runs the packaged message through its typed handler.
    fn run<'a>(
        &'a self,
        context: &'a RouteContext<'a, W>,
        call: Call,
    ) -> RouteFuture<'a, Box<dyn Any>, E>;

    /// Sends a successful value or script failure through the typed responder.
    fn respond(&self, result: Result<Box<dyn Any>, ScriptFailure>);
}

#[doc(hidden)]
trait ErasedRouteFactory<W: WorldContext, E>: Any {
    fn call(&self, message: Box<dyn Any>, responder: Box<dyn Any>) -> Box<dyn RouteCall<W, E>>;
}

struct CallPackage<M: Message, R, W: WorldContext, E> {
    route: Rc<Route<M, R, W, E>>,
    message: RefCell<Option<M>>,
    responder: RefCell<Option<ResponderOf<M>>>,
}

impl<M, R, W, E> RouteCall<W, E> for CallPackage<M, R, W, E>
where
    M: Message + 'static,
    R: 'static,
    W: WorldContext + 'static,
    E: 'static,
{
    fn kind(&self) -> RouteKind {
        self.route.kind
    }

    fn program(&self) -> Option<ProgramId> {
        (self.route.program)(self.message.borrow().as_ref().expect("call has a message"))
    }

    fn run<'a>(
        &'a self,
        context: &'a RouteContext<'a, W>,
        call: Call,
    ) -> RouteFuture<'a, Box<dyn Any>, E> {
        let message = self.message.borrow_mut().take().expect("a route runs once");

        Box::pin(async move {
            self.route
                .run(context, call, message)
                .await
                .map(|value| Box::new(value) as Box<dyn Any>)
        })
    }

    fn respond(&self, result: Result<Box<dyn Any>, ScriptFailure>) {
        let responder = self
            .responder
            .borrow_mut()
            .take()
            .expect("a route responds once");

        match result {
            Ok(value) => self.route.respond(
                responder,
                *value
                    .downcast::<R>()
                    .expect("route response has its declared type"),
            ),
            Err(failure) => self.route.fail(responder, failure),
        }
    }
}

impl<M, R, W, E> ErasedRouteFactory<W, E> for Route<M, R, W, E>
where
    M: Message + Operation + 'static,
    R: 'static,
    W: WorldContext + 'static,
    E: 'static,
{
    fn call(&self, message: Box<dyn Any>, responder: Box<dyn Any>) -> Box<dyn RouteCall<W, E>> {
        Box::new(CallPackage {
            route: Rc::new(Route {
                kind: self.kind,
                handler: self.handler.clone(),
                respond: self.respond,
                fail: self.fail,
                program: self.program,
            }),
            message: RefCell::new(Some(
                *message.downcast::<M>().expect("message matches its route"),
            )),
            responder: RefCell::new(Some(
                *responder
                    .downcast::<ResponderOf<M>>()
                    .expect("responder matches its message"),
            )),
        })
    }
}

/// Routes keyed by their protocol message type.
pub struct RouteTable {
    handlers: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
}

impl Default for RouteTable {
    fn default() -> Self {
        Self {
            handlers: RefCell::new(HashMap::new()),
        }
    }
}

impl RouteTable {
    /// Creates an empty route table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a read-only route.
    pub fn read<M, R, W, E>(
        &self,
        handler: impl AsyncFn(&RouteContext<'_, W>, Call, M) -> RouteResult<R, E> + 'static,
    ) where
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
        W: WorldContext + 'static,
        E: 'static,
    {
        self.insert(RouteKind::Read, handler);
    }

    /// Registers a route that changes the world in one cycle.
    pub fn change<M, R, W, E>(
        &self,
        handler: impl AsyncFn(&RouteContext<'_, W>, Call, M) -> RouteResult<R, E> + 'static,
    ) where
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
        W: WorldContext + 'static,
        E: 'static,
    {
        self.insert(RouteKind::Change, handler);
    }

    /// Registers a route that can wait across world cycles.
    pub fn wait<M, R, W, E>(
        &self,
        handler: impl AsyncFn(&RouteContext<'_, W>, Call, M) -> RouteResult<R, E> + 'static,
    ) where
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
        W: WorldContext + 'static,
        E: 'static,
    {
        self.insert(RouteKind::Wait, handler);
    }

    /// Packages message and responder for their registered route.
    #[must_use]
    pub fn call<M, W, E>(
        &self,
        message: M,
        responder: ResponderOf<M>,
    ) -> Option<Box<dyn RouteCall<W, E>>>
    where
        M: Message + ValidatedDecode + 'static,
        W: WorldContext + 'static,
        E: 'static,
    {
        let factory = self
            .handlers
            .borrow()
            .get(&TypeId::of::<M>())?
            .clone()
            .downcast::<Rc<dyn ErasedRouteFactory<W, E>>>()
            .ok()?;

        Some(factory.call(Box::new(message), Box::new(responder)))
    }

    /// Removes the route of message `M`, returning whether one was registered.
    #[must_use]
    pub fn remove<M: 'static>(&self) -> bool {
        self.handlers
            .borrow_mut()
            .remove(&TypeId::of::<M>())
            .is_some()
    }

    /// Returns whether message `M` has a route.
    #[must_use]
    pub fn contains<M: 'static>(&self) -> bool {
        self.handlers.borrow().contains_key(&TypeId::of::<M>())
    }

    /// Returns the number of registered routes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.handlers.borrow().len()
    }

    /// Returns whether no routes are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.borrow().is_empty()
    }

    fn insert<M, R, W, E>(
        &self,
        kind: RouteKind,
        handler: impl AsyncFn(&RouteContext<'_, W>, Call, M) -> RouteResult<R, E> + 'static,
    ) where
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
        W: WorldContext + 'static,
        E: 'static,
    {
        let handler: Rc<dyn TypedHandler<M, R, W, E>> = Rc::new(HandlerFn(handler));
        let route = Rc::new(Route {
            kind,
            handler,
            respond: |responder, value| responder.respond(value),
            fail: |responder, failure| responder.fail(failure),
            program: |message| message.program(),
        });
        let factory: Rc<dyn ErasedRouteFactory<W, E>> = route;
        let previous = self
            .handlers
            .borrow_mut()
            .insert(TypeId::of::<M>(), Rc::new(factory));

        assert!(previous.is_none(), "a message has two domain routes");
    }
}
