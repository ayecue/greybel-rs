//! Call identity and the services and tree pool supplied to every route.

use std::cell::RefCell;
use std::rc::Rc;

use greyhack_world_programs::protocol::{ObjectId, PrintLine, ProgramId, ProgramPrint};
use greyhack_world_programs::{Process, ProgramTreePool, TreeId};

use crate::CycleAccess;

/// World context shared by routes without depending on the concrete world crate.
pub trait WorldContext {
    /// Typed computer handle stored by connections and routes.
    type Computer: Copy + PartialEq;
    /// Failure of world services called by routes.
    type Error;

    /// Returns module `M` from this world's typed registry.
    fn module<M: 'static>(&self) -> Rc<M>;
}

/// Tree and program that sent one operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Call {
    /// Caller's launch tree.
    pub tree: TreeId,
    /// Program that sent the message.
    pub program: ProgramId,
}

/// Typed program-output lines queued in route execution order.
#[derive(Clone, Default)]
pub struct PrintQueue {
    lines: Rc<RefCell<Vec<ProgramPrint>>>,
}

impl PrintQueue {
    /// Queues `line` for the caller of `call`.
    pub fn push(&self, call: Call, line: PrintLine) {
        self.lines.borrow_mut().push(ProgramPrint {
            program: call.program,
            line,
        });
    }

    /// Removes and returns every queued line in queue order.
    pub fn drain(&self) -> Vec<ProgramPrint> {
        std::mem::take(&mut *self.lines.borrow_mut())
    }
}

/// Services and tree pool supplied to every route.
pub struct RouteContext<'a, W: WorldContext> {
    /// World root and its modules.
    pub modules: Rc<W>,
    /// Program trees of the world.
    pub trees: Rc<ProgramTreePool<W::Computer>>,
    cycles: Option<CycleAccess<'a, W>>,
    prints: PrintQueue,
}

impl<'a, W: WorldContext> RouteContext<'a, W> {
    /// Creates the context of a one-cycle route.
    #[must_use]
    pub fn new(modules: Rc<W>, trees: Rc<ProgramTreePool<W::Computer>>) -> Self {
        Self {
            modules,
            trees,
            cycles: None,
            prints: PrintQueue::default(),
        }
    }

    /// Creates the context of a wait route with cycle access.
    #[must_use]
    pub fn with_cycles(
        modules: Rc<W>,
        trees: Rc<ProgramTreePool<W::Computer>>,
        cycles: CycleAccess<'a, W>,
    ) -> Self {
        Self {
            modules,
            trees,
            cycles: Some(cycles),
            prints: PrintQueue::default(),
        }
    }

    /// Uses `prints` for lines queued by this route and its wait-route cycles.
    #[must_use]
    pub fn with_prints(mut self, prints: PrintQueue) -> Self {
        self.prints = prints;

        self
    }

    /// Queues a typed output line for the caller of `call`.
    pub fn print(&self, call: Call, line: PrintLine) {
        self.prints.push(call, line);
    }

    /// Returns cycle access for a wait route.
    #[must_use]
    pub fn cycles(&self) -> CycleAccess<'a, W> {
        self.cycles.expect("only wait routes receive cycle access")
    }

    /// Returns the caller's session object if its program is active.
    #[must_use]
    pub fn session(&self, call: Call) -> Option<ObjectId> {
        self.trees.get(call.tree)?.session(call.program)
    }

    /// Returns the root process of the caller's tree.
    #[must_use]
    pub fn root_process(&self, call: Call) -> Option<Process<W::Computer>> {
        self.trees.get(call.tree)?.root_process()
    }

    /// Returns the process of the terminal containing the caller.
    #[must_use]
    pub fn command_process(&self, call: Call) -> Option<Process<W::Computer>> {
        self.trees.get(call.tree)?.command_process(call.program)
    }

    /// Returns the last shell computer of the caller's tree.
    #[must_use]
    pub fn last_shell(&self, call: Call, player: W::Computer) -> W::Computer {
        self.trees
            .get(call.tree)
            .map_or(player, |tree| tree.last_shell(player))
    }

    /// Returns the launch depth of `program`.
    #[must_use]
    pub fn depth(&self, call: Call, program: ProgramId) -> u32 {
        self.trees
            .get(call.tree)
            .map_or(0, |tree| tree.depth(program))
    }

    /// Returns whether the caller's tree is in safe-terminal mode.
    #[must_use]
    pub fn safe_terminal(&self, call: Call) -> bool {
        self.trees
            .get(call.tree)
            .is_some_and(|tree| tree.safe_terminal())
    }

    /// Returns terminal computers from the oldest to the innermost.
    #[must_use]
    pub fn terminal_computers(&self, call: Call) -> Vec<W::Computer> {
        self.trees
            .get(call.tree)
            .map_or_else(Vec::new, |tree| tree.terminals.as_slice().to_vec())
    }

    /// Marks the caller's terminal computer as rebooted.
    pub fn reboot_terminal(&self, call: Call) {
        self.trees
            .get_mut(call.tree)
            .expect("the indexed tree exists")
            .effects_mut()
            .terminal_rebooted = true;
    }
}
