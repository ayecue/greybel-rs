use std::cell::{Cell, RefCell};
use std::rc::Rc;

use greybel_interpreter::{Pin, Value};

use super::object::Access;
use super::session::Session;
use super::terminal::TerminalOutput;
use crate::protocol::{ProgramId, ProgramSession};
use crate::runtime::ServerConnection;

/// Context-local game state.
/// Contexts in one launch tree share the server connection, custom object, launch list, VM, and heap, mirroring Grey Hack's shared terminal `HelperScript`.
#[derive(Clone)]
pub struct Game {
    /// Game server that owns the world.
    pub server: ServerConnection,
    /// Process identity used by the script.
    pub program: ProgramId,
    pub session: Session,
    /// Controls sandbox-only `include_lib("admin.so")` access; enabled by default.
    pub admin: bool,
    /// Initialized custom object shared by the launch tree.
    pub(crate) custom_object: Rc<RefCell<Option<Value>>>,
    /// Current custom object's heap value and GC root.
    pub(crate) custom_pin: Rc<RefCell<Option<(Value, Pin)>>>,
    /// Active launched-context program IDs, ordered from the root child to the current context.
    pub(crate) launches: Rc<RefCell<Vec<ProgramId>>>,
    /// Marks a server-side SSH encode/decode program running for an encrypted login.
    pub(crate) encrypts_login: bool,
    /// Launch-tree terminal output.
    pub(crate) output: Rc<TerminalOutput>,
    /// Parent resume directory after this program calls `cd`.
    /// `PlayerIntrinsics.cd` updates the last program state's `currentPath`; runtime-started programs have no parent directory.
    pub(crate) launcher_folder: Option<Rc<RefCell<Option<String>>>>,
    /// Set by `exit` in a program that `start_terminal` started, which ends its terminal with `exit`
    /// (`PlayerUtils.ExitCommand`) rather than by running out of code.
    pub(crate) terminal_exit: Option<Rc<Cell<bool>>>,
}

impl Game {
    /// Creates state from a server-created `session`, starting in the user's home folder and printing to `output`.
    #[must_use]
    pub fn new(
        server: ServerConnection,
        session: ProgramSession,
        output: Rc<TerminalOutput>,
    ) -> Self {
        Self {
            program: session.program,
            session: Session {
                shell: Access::new(server.object(session.shell)),
                user: session.user,
                current_path: session.home,
                launch_path: session.path.clone(),
                program_path: session.path,
            },
            server,
            admin: true,
            custom_object: Rc::new(RefCell::new(None)),
            custom_pin: Rc::default(),
            launches: Rc::default(),
            encrypts_login: false,
            output,
            launcher_folder: None,
            terminal_exit: None,
        }
    }

    /// Creates state for child process `program`, sharing the server, admin setting, custom object, and launch list.
    #[must_use]
    pub(crate) fn child(&self, session: Session, program: ProgramId) -> Self {
        Self {
            server: self.server.clone(),
            program,
            session,
            admin: self.admin,
            custom_object: Rc::clone(&self.custom_object),
            custom_pin: Rc::clone(&self.custom_pin),
            launches: Rc::clone(&self.launches),
            encrypts_login: self.encrypts_login,
            output: Rc::clone(&self.output),
            launcher_folder: None,
            terminal_exit: None,
        }
    }
}
