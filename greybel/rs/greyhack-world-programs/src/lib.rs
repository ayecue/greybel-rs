#![deny(clippy::await_holding_refcell_ref)]

//! Program launch trees, typed object handles, and tree-local state shared by world routers.

mod error;
mod object;
mod program;
mod state;
mod trees;

pub mod protocol;

pub use error::{RouteError, RouteResult};
pub use object::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable, ObjectTableHandle,
};
pub use program::{
    ClientProgramEnd, Process, Program, ProgramEnd, ProgramKind, ProgramTree,
    RouteEffect as RouteEffects, Stage, TerminalClose, TerminalInfo, TerminalStack,
};
pub use state::StateMap;
pub use trees::{ProgramTreePool, TreeId};
