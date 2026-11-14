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
    CycleAccess, CycleAccess as Cycles, CycleDriver, CycleDriverRef, CycleFuture, ErasedCycleCall,
    ErasedWorldCall,
};
pub use greyhack_world_programs::{
    ClientProgramEnd, Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable,
    ObjectTableHandle, Process, Program, ProgramEnd, ProgramKind, ProgramTree, ProgramTreePool,
    ProgramTrees, RouteEffects, RouteError, RouteResult, Stage, StateMap, TerminalClose,
    TerminalInfo, TerminalStack, TreeId, protocol,
};
pub use operation::Operation;
pub use routes::{Client, OperationResponse, ResponderOf};
pub use table::{Route, RouteCall, RouteFuture, RouteKind, RouteTable};
