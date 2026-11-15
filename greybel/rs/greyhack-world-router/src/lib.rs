#![deny(clippy::await_holding_refcell_ref)]

//! World-protocol bindings and the transport-independent state used to route operations between a runtime
//! and its world.

mod context;
mod cycles;
mod operation;
mod routes;
mod table;

pub use context::{Call, PrintQueue, RouteContext, WorldContext};
pub use cycles::{
    CycleAccess, CycleDriver, CycleDriverRef, CycleFuture, ErasedCycleCall, ErasedWorldCall,
};
pub use operation::Operation;
pub use routes::{OperationResponse, ResponderOf};
pub use table::{Route, RouteCall, RouteFuture, RouteKind, RouteTable};
