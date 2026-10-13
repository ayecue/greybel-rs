//! Child-program start: state a parent hands to a server-started child, and entering its context.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use greybel_interpreter::{ContextEntry, NativeOutput, PrintOptions, Value, Vm};

use super::cache::{Code, launch_key};
use super::child_end::ChildEnd;
use super::{CachedCode, LaunchCache};
use crate::game::client::failure::failure_text;
use crate::game::client::object::Access;
use crate::game::client::server::server;
use crate::game::client::state::reset_custom_object;
use crate::game::{Game, Session, game};
use crate::protocol::{ClientEnd, Failure, ProgramChannel, ProgramSession, SourceMap};
use crate::runtime::ChildRunner;

/// Child start directory and the values returned by `launch_path` and `program_path` within it.
pub(in super::super) struct ChildLocation {
    pub(in super::super) current: String,
    pub(in super::super) launch: String,
    pub(in super::super) program: String,
}

/// Parent-provided child state: game state, runner, compiled-code cache, and output.
pub(in super::super) struct ChildStart {
    pub(in super::super) game: Game,
    pub(in super::super) runner: Option<ChildRunner>,
    pub(in super::super) launches: Rc<RefCell<std::collections::HashMap<u64, CachedCode>>>,
    pub(in super::super) stdout: Rc<dyn greybel_interpreter::Stdout>,
    /// Whether the child receives a new custom object, as in `start_terminal`.
    pub(in super::super) resets_custom_object: bool,
    /// Whether the child is the terminal `start_terminal` starts.
    pub(in super::super) terminal: bool,
}

impl ChildStart {
    /// Returns child-start state and the game server channel for the program running as `vm`, or `None` without a game session.
    pub(in super::super) fn begin(vm: &mut Vm) -> Option<(Self, Rc<dyn ProgramChannel>)> {
        let (Ok(game), Ok(server)) = (game(vm).cloned(), server(vm)) else {
            return None;
        };

        Some((Self::of(vm, game), server))
    }

    /// Prints `failure` as `intrinsic`'s text on the terminal and returns false, the result of a failed launch.
    pub(in super::super) fn print_failure(
        &self,
        intrinsic: &'static str,
        failure: &Failure,
    ) -> NativeOutput {
        match failure_text(intrinsic, failure) {
            Ok(text) => {
                self.stdout.print(&text, PrintOptions::default());

                NativeOutput::new(false)
            }
            Err(error) => NativeOutput::new(Err::<Value, _>(error)),
        }
    }

    /// Returns child-start state for the program running as `vm` with `game`.
    pub(in super::super) fn of(vm: &mut Vm, game: Game) -> Self {
        Self {
            runner: vm.host_data::<ChildRunner>().cloned(),
            launches: Rc::clone(&vm.host_data_or_insert_with(LaunchCache::default).0),
            stdout: vm.stdout().clone(),
            resets_custom_object: false,
            terminal: false,
            game,
        }
    }

    /// Enters server-started child `session`, running `source` with `params` at `paths`.
    /// Returns the starting call's result after the child exits.
    pub(in super::super) async fn enter(
        self,
        session: ProgramSession,
        paths: ChildLocation,
        source: String,
        source_map: SourceMap,
        params: Vec<String>,
    ) -> NativeOutput {
        let server = Rc::clone(self.game.server.channel());
        let Some(runner) = self.runner else {
            return NativeOutput::new(true);
        };
        let key = launch_key(&session.path, &source);
        let hit = self.launches.borrow().get(&key).cloned();
        let program = session.program;
        let path = session.path.clone();
        // Child `cd` updates the directory where this program resumes (`PlayerIntrinsics.cd` sets the last program state's `currentPath`).
        let launcher_folder = Rc::new(RefCell::new(None));
        let mut child = self.game.child(
            Session {
                shell: Access::new(self.game.server.object(session.shell)),
                user: session.user,
                current_path: paths.current,
                launch_path: paths.launch,
                program_path: paths.program,
            },
            program,
        );

        child.launcher_folder = Some(Rc::clone(&launcher_folder));

        let ends_terminal = self.terminal.then(Rc::<Cell<bool>>::default);

        child.terminal_exit.clone_from(&ends_terminal);

        let end = ChildEnd {
            program,
            server,
            stdout: self.stdout,
            source_map: hit
                .as_ref()
                .map_or_else(|| (&source_map).into(), |hit| hit.source_map.clone()),
            launcher_folder,
            ends_terminal,
        };

        let code = match hit {
            Some(hit) => Code::Entry(hit.entry),
            None => match runner.compile(&path, source).await {
                Ok(program) => Code::Program(Rc::unwrap_or_clone(program)),
                Err(error) => {
                    end.stdout
                        .print(&error.to_string(), PrintOptions::default());
                    child.server.forget(program);

                    end.send(ClientEnd::Error, false).await;

                    return NativeOutput::with_vm(|vm| {
                        let _ = reset_custom_object(vm);

                        false
                    });
                }
            },
        };
        let launches = self.launches;
        let resets_custom_object = self.resets_custom_object;

        NativeOutput::enter(move |vm| {
            if resets_custom_object {
                let _ = reset_custom_object(vm);
            }

            let entry = match code {
                Code::Entry(entry) => entry,
                Code::Program(code) => {
                    let entry = vm.add_program(code);
                    let code_entry = CachedCode {
                        entry,
                        source_map: end.source_map.clone(),
                    };

                    launches.borrow_mut().insert(key, code_entry);

                    entry
                }
            };

            child.launches.borrow_mut().push(program);

            Ok(
                ContextEntry::new(entry, params, move |vm, exit| end.after(vm, exit))
                    .with_host_data(child),
            )
        })
    }
}
