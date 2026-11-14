//! Programs owned by one launch tree, tracked until the runtime acknowledges termination. Ended program IDs
//! remain as tombstones until the connection closes so late operations cannot reach a reused ID.

use crate::protocol::{EndReason, ObjectId, ProgramId};
use crate::state::StateMap;

/// Process key containing computer ID and PID.
pub type Process<C> = (C, u32);

/// Program lifecycle stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Running program that accepts operations.
    Active,
    /// Server-terminated program ([`EndReason`]) awaiting runtime acknowledgment.
    Termination,
    /// Runtime-ended program retaining only its ID to reject late operations.
    Tombstone,
}

/// Where a `start_terminal` program runs and the bounces its exit compares (`HelperScript.CloseRemotePrograms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalInfo<C> {
    /// The shell's computer, where the terminal runs.
    pub host: C,
    /// Bounces of the terminal's shell (`MainClass`'s `startNumBounces` while the terminal ends).
    pub bounces: u32,
    /// Bounces of the program that started the terminal, which resumes when it ends.
    pub launcher_bounces: u32,
}

/// Whether a program is a script or a terminal started by `start_terminal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramKind<C> {
    /// Runtime-started or launched script.
    Script,
    /// Terminal with its host and bounce information.
    Terminal(TerminalInfo<C>),
}

/// Program process, shell session, parent launch, and launch depth.
pub struct Program<C> {
    /// Program ID, unique for the lifetime of its server.
    pub id: ProgramId,
    /// Absent for `start_terminal` programs, which have no process, and launched programs whose process
    /// `start_terminal` closed.
    pub process: Option<Process<C>>,
    /// Shell object that identifies the program's user and computer.
    pub session: ObjectId,
    /// Program that launched this program.
    pub parent: Option<ProgramId>,
    /// Number of launches between this program and its runtime-started root.
    pub depth: u32,
    /// Current lifecycle stage.
    pub stage: Stage,
    /// Script or terminal program.
    pub kind: ProgramKind<C>,
}

impl<C> Program<C> {
    fn root(id: ProgramId, process: Option<Process<C>>, session: ObjectId) -> Self {
        Self {
            id,
            process,
            session,
            parent: None,
            depth: 0,
            stage: Stage::Active,
            kind: ProgramKind::Script,
        }
    }

    /// Returns terminal information when this program was started by `start_terminal`.
    #[must_use]
    pub const fn terminal_info(&self) -> Option<TerminalInfo<C>>
    where
        C: Copy,
    {
        match self.kind {
            ProgramKind::Terminal(info) => Some(info),
            ProgramKind::Script => None,
        }
    }
}

/// Terminal computers of one tree (`HelperScript.pendingShell`).
#[derive(Debug)]
pub struct TerminalStack<C> {
    computers: Vec<C>,
}

impl<C> Default for TerminalStack<C> {
    fn default() -> Self {
        Self {
            computers: Vec::new(),
        }
    }
}

impl<C: Copy> TerminalStack<C> {
    /// Appends a terminal computer as `HelperScript.AddPendingShell` does.
    pub fn push(&mut self, computer: C) {
        self.computers.push(computer);
    }

    /// Removes the last shell as `MainClass.UpdateTerminalClient(removeLastShell: true)` does.
    pub fn remove_last(&mut self) {
        self.computers.pop();
    }

    /// Returns the innermost terminal computer, or `player` when the tree has none
    /// (`HelperScript.GetLastShell`).
    #[must_use]
    pub fn last_shell(&self, player: C) -> C {
        self.computers.last().copied().unwrap_or(player)
    }

    /// Returns terminal computers from the oldest to the innermost.
    #[must_use]
    pub fn as_slice(&self) -> &[C] {
        &self.computers
    }
}

/// What a terminal's exit closes: the terminal on `remote`, started from `source`
/// (`HelperScript.CloseRemotePrograms`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalClose<C> {
    /// The computer the terminal's connection came from.
    pub source: C,
    /// The computer the terminal ran on.
    pub remote: C,
}

/// Server-side program termination before the server adds its notification channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramEnd<C> {
    /// Ended program.
    pub program: ProgramId,
    /// Why the program ended.
    pub reason: EndReason,
    /// Process the server must end.
    pub process: Option<Process<C>>,
}

/// Result of handling a runtime's end message.
pub struct ClientProgramEnd<C> {
    /// Running process that the caller must remove.
    pub process: Option<Process<C>>,
    /// Running child programs terminated with the parent.
    pub children: Vec<ProgramEnd<C>>,
}

/// Route effects applied by the server before it responds.
#[derive(Debug, Default)]
pub struct RouteEffect {
    /// The terminal's own computer rebooted (`GreyInterpreter.exitBashReboot`).
    pub terminal_rebooted: bool,
}

/// Runtime-started program and its descendants, which share one runtime heap and one object table.
pub struct ProgramTree<C> {
    root: ProgramId,
    connection: u32,
    programs: Vec<Program<C>>,
    /// Pending terminal shells shared by the tree.
    pub terminals: TerminalStack<C>,
    /// Computers on which the tree started terminals (`Proceso.remoteMainTerminalPID`).
    remote_terminals: Vec<C>,
    /// Whether the root is typed into the safe-mode terminal (`GreyInterpreter.isSafeTerminal`).
    safe_terminal: bool,
    states: StateMap,
    effects: RouteEffect,
}

impl<C: Copy + PartialEq> ProgramTree<C> {
    /// Creates a tree containing its runtime-started root program.
    #[must_use]
    pub fn new(
        connection: u32,
        root: ProgramId,
        process: Option<Process<C>>,
        session: ObjectId,
        safe_terminal: bool,
    ) -> Self {
        Self {
            root,
            connection,
            programs: vec![Program::root(root, process, session)],
            terminals: TerminalStack::default(),
            remote_terminals: Vec::new(),
            safe_terminal,
            states: StateMap::default(),
            effects: RouteEffect::default(),
        }
    }

    /// Returns the runtime connection that owns this tree.
    #[must_use]
    pub const fn connection(&self) -> u32 {
        self.connection
    }

    /// Returns the root program ID.
    #[must_use]
    pub const fn root_id(&self) -> ProgramId {
        self.root
    }

    /// Returns the root program.
    #[must_use]
    pub fn root(&self) -> &Program<C> {
        self.program(self.root)
            .expect("a program tree contains its root")
    }

    /// Returns the newest running program on the launch stack.
    #[must_use]
    pub fn current(&self) -> Option<&Program<C>> {
        self.programs
            .iter()
            .rev()
            .find(|program| program.stage == Stage::Active)
    }

    /// Returns program `id`, including its termination stage or tombstone.
    #[must_use]
    pub fn program(&self, id: ProgramId) -> Option<&Program<C>> {
        self.programs.iter().find(|program| program.id == id)
    }

    /// Returns mutable program `id`, including its termination stage or tombstone.
    #[must_use]
    pub fn program_mut(&mut self, id: ProgramId) -> Option<&mut Program<C>> {
        self.programs.iter_mut().find(|program| program.id == id)
    }

    /// Returns active program `id`.
    #[must_use]
    pub fn active(&self, id: ProgramId) -> Option<&Program<C>> {
        self.program(id)
            .filter(|program| program.stage == Stage::Active)
    }

    /// Pushes a launched script or terminal onto the tree. Launch depth is one greater than its launcher;
    /// safe-terminal state is shared by all of the tree (`GreyInterpreter.isSafeTerminal`).
    pub fn push(
        &mut self,
        id: ProgramId,
        process: Option<Process<C>>,
        session: ObjectId,
        parent: ProgramId,
        kind: ProgramKind<C>,
    ) {
        let depth = self.program(parent).map_or(0, |parent| parent.depth + 1);

        self.programs.push(Program {
            id,
            process,
            session,
            parent: Some(parent),
            depth,
            stage: Stage::Active,
            kind,
        });
    }

    /// Returns whether the tree runs in the safe-mode terminal.
    #[must_use]
    pub const fn safe_terminal(&self) -> bool {
        self.safe_terminal
    }

    /// Marks the root as typed into the safe-mode terminal.
    pub fn set_safe_terminal(&mut self) {
        self.safe_terminal = true;
    }

    /// Adds `computer` to the terminals the tree started, which `ShellIntrinsics.start_terminal` does before
    /// it checks anything (`Proceso.AddMainTerminalRemote`).
    pub fn add_remote_terminal(&mut self, computer: C) {
        self.remote_terminals.push(computer);
    }

    /// Returns the innermost terminal `program` runs in: itself when it is one, otherwise the nearest program
    /// that launched it (`GreyInterpreter.hostData`'s last VM that is a bash).
    #[must_use]
    pub fn terminal_of(&self, program: ProgramId) -> Option<ProgramId> {
        let mut id = program;

        loop {
            let entry = self.program(id)?;

            if entry.terminal_info().is_some() {
                return Some(id);
            }

            id = entry.parent?;
        }
    }

    /// Returns the program that started `terminal`, whose state `MainClass` takes off the pending VMs when
    /// the terminal ends (`HelperScript.GetLastVM`).
    #[must_use]
    pub fn launcher_of(&self, terminal: ProgramId) -> Option<ProgramId> {
        self.program(terminal).and_then(|entry| entry.parent)
    }

    /// Returns the host of `program` when it is a terminal.
    #[must_use]
    pub fn terminal_host(&self, program: ProgramId) -> Option<C> {
        self.program(program)
            .and_then(Program::terminal_info)
            .map(|info| info.host)
    }

    /// Returns the computer of the last shell the tree has, or `player` when it has none
    /// (`HelperScript.GetLastShell`).
    #[must_use]
    pub fn last_shell(&self, player: C) -> C {
        self.terminals.last_shell(player)
    }

    /// Ends the last terminal the tree started when `program` is a terminal that exited, and returns what it
    /// closes (`HelperScript.CloseRemotePrograms`). A terminal on the computer it came from, with the bounces
    /// of its launcher, closes nothing. `player` is where the first terminal's connection comes from.
    pub fn exit_terminal(&mut self, program: ProgramId, player: C) -> Option<TerminalClose<C>> {
        let info = self.program(program)?.terminal_info()?;

        self.terminals.remove_last();

        let remote = self.remote_terminals.pop()?;
        let source = self.remote_terminals.last().copied().unwrap_or(player);

        (remote != source || info.bounces != info.launcher_bounces)
            .then_some(TerminalClose { source, remote })
    }

    /// Returns the process of active `program` when a terminal launched it, the command the terminal runs
    /// (`HelperScript.subPID`); otherwise `None`.
    #[must_use]
    pub fn command_process(&self, program: ProgramId) -> Option<Process<C>> {
        let entry = self.active(program)?;
        let launcher = self.program(entry.parent?)?;

        matches!(launcher.kind, ProgramKind::Terminal(_))
            .then_some(entry.process)
            .flatten()
    }

    /// Returns the session of active `program`.
    #[must_use]
    pub fn session(&self, program: ProgramId) -> Option<ObjectId> {
        self.active(program).map(|entry| entry.session)
    }

    /// Makes `session` the shell session of active `program`.
    pub fn set_session(&mut self, program: ProgramId, session: ObjectId) {
        if let Some(entry) = self
            .program_mut(program)
            .filter(|entry| entry.stage == Stage::Active)
        {
            entry.session = session;
        }
    }

    /// Removes and returns the process of active launched `program`, closed by `start_terminal`
    /// (`ShellIntrinsics.start_terminal`). The program continues without a process.
    pub fn take_launched_process(&mut self, program: ProgramId) -> Option<Process<C>> {
        self.program_mut(program)
            .filter(|entry| entry.stage == Stage::Active && entry.parent.is_some())
            .and_then(|entry| entry.process.take())
    }

    /// Returns the root process (`GreyInterpreter.PID`), or `None` when absent.
    #[must_use]
    pub fn root_process(&self) -> Option<Process<C>> {
        self.root().process
    }

    /// Returns launch depth of `program`, with 0 for runtime-started programs.
    #[must_use]
    pub fn depth(&self, program: ProgramId) -> u32 {
        self.program(program).map_or(0, |entry| entry.depth)
    }

    /// Checks for programs that are not tombstones.
    #[must_use]
    pub fn has_programs(&self) -> bool {
        self.programs
            .iter()
            .any(|program| program.stage != Stage::Tombstone)
    }

    /// Handles runtime termination of `program`. Active programs end with active descendants; terminating
    /// programs and their terminating descendants become tombstones without separate child acknowledgments.
    /// Returns `None` for tombstones.
    pub fn pop(&mut self, program: ProgramId) -> Option<ClientProgramEnd<C>> {
        let entry = self.program_mut(program)?;

        if entry.stage == Stage::Tombstone {
            return None;
        }

        let running = entry.stage == Stage::Active;

        entry.stage = Stage::Tombstone;

        if !running {
            self.acknowledge_children(program.id);

            return Some(ClientProgramEnd {
                process: None,
                children: Vec::new(),
            });
        }

        let process = entry.process;
        let children = self.end_children(&[program.id]);

        Some(ClientProgramEnd { process, children })
    }

    /// Converts all terminating descendants of `parent` to [`Stage::Tombstone`].
    fn acknowledge_children(&mut self, parent: u32) {
        let mut parents = vec![parent];

        while let Some(parent) = parents.pop() {
            for program in &mut self.programs {
                if program.parent == Some(ProgramId { id: parent })
                    && program.stage == Stage::Termination
                {
                    program.stage = Stage::Tombstone;
                    parents.push(program.id.id);
                }
            }
        }
    }

    /// Lists active program processes hosted on any of `computers`.
    #[must_use]
    pub fn processes_on(&self, computers: &[C]) -> Vec<(ProgramId, Process<C>)> {
        self.programs
            .iter()
            .filter(|program| program.stage == Stage::Active)
            .filter_map(|program| {
                program
                    .process
                    .filter(|process| computers.contains(&process.0))
                    .map(|process| (program.id, process))
            })
            .collect()
    }

    /// Terminates `items` programs and their active descendants, returning all termination records.
    pub fn take_terminations(&mut self, items: &[(ProgramId, EndReason)]) -> Vec<ProgramEnd<C>> {
        let mut terminations: Vec<ProgramEnd<C>> = items
            .iter()
            .filter_map(|&(program, reason)| self.terminate(program, reason))
            .collect();
        let ids: Vec<u32> = terminations
            .iter()
            .map(|termination| termination.program.id)
            .collect();

        terminations.extend(self.end_children(&ids));

        terminations
    }

    /// Terminates all active descendants of `parents` with [`EndReason::ParentEnd`].
    fn end_children(&mut self, parents: &[u32]) -> Vec<ProgramEnd<C>> {
        let mut parents = parents.to_vec();
        let mut terminations = Vec::new();
        let mut index = 0;

        while let Some(&parent) = parents.get(index) {
            let children: Vec<u32> = self
                .programs
                .iter()
                .filter(|program| {
                    program.parent == Some(ProgramId { id: parent })
                        && program.stage == Stage::Active
                })
                .map(|child| child.id.id)
                .collect();

            for child in children {
                if let Some(termination) =
                    self.terminate(ProgramId { id: child }, EndReason::ParentEnd)
                {
                    terminations.push(termination);
                    parents.push(child);
                }
            }

            index += 1;
        }

        terminations
    }

    /// Moves active program `id` to [`Stage::Termination`] with `reason`.
    fn terminate(&mut self, id: ProgramId, reason: EndReason) -> Option<ProgramEnd<C>> {
        let program = self
            .program_mut(id)
            .filter(|program| program.stage == Stage::Active)?;

        program.stage = Stage::Termination;

        Some(ProgramEnd {
            program: id,
            reason,
            process: program.process,
        })
    }

    /// Returns all active processes in the tree.
    #[must_use]
    pub fn active_processes(&self) -> Vec<Process<C>> {
        self.programs
            .iter()
            .filter(|program| program.stage == Stage::Active)
            .filter_map(|program| program.process)
            .collect()
    }

    /// Returns domain state of `T`, creating its default value on first access.
    pub fn state<T: Default + Send + 'static>(&mut self) -> &T {
        self.states.state::<T>()
    }

    /// Returns mutable domain state of `T`, creating its default value on first access.
    pub fn state_mut<T: Default + Send + 'static>(&mut self) -> &mut T {
        self.states.state_mut::<T>()
    }

    /// Returns the tree's state map for a registered work owner.
    pub fn state_map_mut(&mut self) -> &mut StateMap {
        &mut self.states
    }

    /// Starts collecting effects for one route.
    pub fn begin_route(&mut self) {
        self.effects = RouteEffect::default();
    }

    /// Returns mutable effects of the current route.
    pub fn effects_mut(&mut self) -> &mut RouteEffect {
        &mut self.effects
    }

    /// Takes the effects produced by the current route.
    pub fn take_effects(&mut self) -> RouteEffect {
        std::mem::take(&mut self.effects)
    }
}
