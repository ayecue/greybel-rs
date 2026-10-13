//! Completes parent `launch` calls and reports child-context exits to the game server.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use greybel_interpreter::{
    ContextExit, NativeFuture, NativeOutput, PrintOptions, RuntimeError, RuntimeErrorKind, Stdout,
    Vm,
};
use greybel_source_map::{Location, SourceMap};

use crate::game::client::state::reset_custom_object;
use crate::game::{game, session_mut};
use crate::protocol::{ClientEnd, EndProgram, ProgramChannel, ProgramId};

/// State needed to report a launched program's exit and finish its parent's native call.
pub(super) struct ChildEnd {
    pub(super) program: ProgramId,
    pub(super) server: Rc<dyn ProgramChannel>,
    pub(super) stdout: Rc<dyn Stdout>,
    /// Maps program error lines to source locations.
    pub(super) source_map: SourceMap,
    /// Last directory selected by child `cd`; the parent resumes there.
    pub(super) launcher_folder: Rc<RefCell<Option<String>>>,
    /// For a terminal, whether it ended with `exit`.
    pub(super) ends_terminal: Option<Rc<Cell<bool>>>,
}

impl ChildEnd {
    /// Handles `exit` after resuming the parent context.
    /// A runtime failure following server-side child termination is acknowledged rather than treated as a crash; a crash prints through Grey Hack's terminal format and resets the custom object (`MainClass`'s `scriptCrash`).
    /// Once the child compiles, `launch` returns true for every exit mode (`ShellIntrinsics`' `launch` returns `ResetVM`'s result).
    /// The returned future reports the exit; dropping it is safe because the server also acknowledges programs launched by a terminated program.
    pub(super) fn after(self, vm: &mut Vm, exit: ContextExit) -> NativeFuture {
        let reason = match exit {
            ContextExit::Completion
                if self
                    .ends_terminal
                    .as_ref()
                    .is_some_and(|ends_terminal| ends_terminal.get()) =>
            {
                ClientEnd::Exited
            }
            ContextExit::Completion => ClientEnd::Finished,
            ContextExit::Termination => ClientEnd::Acknowledged,
            ContextExit::Failure(_)
                if game(vm).is_ok_and(|game| game.server.has_ended(self.program)) =>
            {
                ClientEnd::Acknowledged
            }
            ContextExit::Failure(error) => {
                self.stdout.print(
                    &crash_report(&error, &self.source_map),
                    PrintOptions::default(),
                );

                let _ = reset_custom_object(vm);

                ClientEnd::Error
            }
        };

        if let Ok(game) = game(vm) {
            game.launches.borrow_mut().pop();
            game.server.forget(self.program);
        }

        if let Some(folder) = self.launcher_folder.borrow_mut().take()
            && let Ok(session) = session_mut(vm)
        {
            session.current_path = folder;
        }

        Box::pin(self.send(reason, true))
    }

    /// Sends `reason` to the game server, awaits its response, and returns `success` from `launch`.
    pub(super) async fn send(self, reason: ClientEnd, success: bool) -> NativeOutput {
        let end = self.server.send_end_program(&EndProgram {
            program: self.program,
            reason,
        });

        if let Ok(end) = end {
            let _ = end.await;
        }

        NativeOutput::new(success)
    }
}

/// Formats `error` like Grey Hack's terminal.
/// Uses source file and line when `source_map` resolves the line, only the line for empty source, or the program path when the map has no file entry.
fn crash_report(error: &RuntimeError, source_map: &SourceMap) -> String {
    if error.kind == RuntimeErrorKind::MemoryLimit {
        return format!("<b>{error}</b>");
    }

    let location = error
        .stack
        .first()
        .map(|at| match source_map.locate(at.line) {
            Some(Location { source: "", line }) => format!(" [line {line}]"),
            Some(Location { source, line }) => format!(" [{source} line {line}]"),
            None => format!(" [{} line {}]", at.path, at.line),
        })
        .unwrap_or_default();

    format!("<color=red>Runtime Error: {error}{location}</color>")
}
