//! Batches program-terminal output as Grey Hack does (`GreyInterpreter.terminalLines`).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use greybel_interpreter::{Clock, FromVm, PrintOptions, Stdout, Vm};

use super::game::Game;

/// Running-program output batch interval (`GreyInterpreter.ShouldSendPrint`).
const SEND_INTERVAL: Duration = Duration::from_millis(200);
/// Maximum batch length in UTF-16 units, C#'s `string.Length`; excess oldest characters are discarded
/// (`GreyInterpreter.AddPendingOutput`).
const MAX_CHARACTERS: usize = 160_000;

/// Grey Hack-compatible buffered terminal output.
/// While running, batches flush every 200 ms; they also flush before input, timer, and game waits, and at exit.
/// A replacing print flushes the previous batch, then sends a clear plus the new batch and its later output (`HelperScript.AddOutput`, `GreyInterpreter.SendPendingPrint`).
/// Launched programs append to their launcher's batch because they share its interpreter.
pub struct TerminalOutput {
    stdout: Rc<dyn Stdout>,
    clock: Rc<dyn Clock>,
    batch: RefCell<Batch>,
}

struct Batch {
    lines: Vec<String>,
    replaces: bool,
    /// Last start time of the 200 ms send interval on the VM clock.
    last_send: Duration,
}

impl TerminalOutput {
    /// Creates output for a program starting now, displayed through `stdout`.
    #[must_use]
    pub fn new(stdout: Rc<dyn Stdout>, clock: Rc<dyn Clock>) -> Self {
        let last_send = clock.now();

        Self {
            stdout,
            clock,
            batch: RefCell::new(Batch {
                lines: Vec::new(),
                replaces: false,
                last_send,
            }),
        }
    }

    /// Flushes the batch to the terminal (`GreyInterpreter.SendPendingPrint`).
    pub fn send(&self) {
        let mut batch = self.batch.borrow_mut();

        if batch.lines.is_empty() {
            return;
        }

        let text = batch.lines.join("\n");
        let replaces = batch.replaces;

        batch.lines.clear();
        batch.replaces = false;
        drop(batch);

        // Replacing batches replace all displayed terminal text (`TerminalListAdapter.AddText`).
        if replaces {
            self.stdout.clear();
        }

        self.stdout.print(&text, PrintOptions::default());
    }

    /// Flushes after 200 ms since the previous timed send, matching Grey Hack's between-step run-loop check.
    fn send_on_time(&self) {
        let now = self.clock.now();
        let due = now.saturating_sub(self.batch.borrow().last_send) > SEND_INTERVAL;

        if due {
            self.send();
            self.batch.borrow_mut().last_send = now;
        }
    }
}

impl Stdout for TerminalOutput {
    fn print(&self, text: &str, options: PrintOptions) {
        self.send_on_time();

        if options.replace && !self.batch.borrow().replaces {
            self.send();
            self.batch.borrow_mut().replaces = true;
        }

        let mut batch = self.batch.borrow_mut();

        batch.lines.push(text.to_owned());

        let mut characters: usize = batch
            .lines
            .iter()
            .map(|line| line.encode_utf16().count())
            .sum();

        while characters > MAX_CHARACTERS && !batch.lines.is_empty() {
            let excess = characters - MAX_CHARACTERS;
            let first = &mut batch.lines[0];
            let length = first.encode_utf16().count();

            if length > excess {
                let mut units = 0;
                let cut = first
                    .char_indices()
                    .find_map(|(index, character)| {
                        let start = units;

                        units += character.len_utf16();

                        (start >= excess).then_some((index, start))
                    })
                    .unwrap_or((first.len(), length));

                first.drain(..cut.0);
                characters -= cut.1;
            } else {
                characters -= length;
                batch.lines.remove(0);
            }
        }
    }

    /// Discards the batch and clears the terminal (`GreyInterpreter.SendClearClient`).
    fn clear(&self) {
        self.batch.borrow_mut().lines.clear();
        self.stdout.clear();
    }

    /// Delegates flushes; batches reach the terminal only at Grey Hack's normal send points.
    fn flush(&self) {
        self.stdout.flush();
    }

    /// Flushes a due batch while the program runs, matching Grey Hack's run loop.
    fn flush_if_due(&self) {
        self.send_on_time();
        self.stdout.flush_if_due();
    }
}

/// Host parameter for intrinsics that flush before time, input, transfer, decipher, or capture waits, or emulate terminal display.
pub(crate) struct Terminal {
    /// Grey Hack's output batch.
    output: Option<Rc<TerminalOutput>>,
    /// Terminal-text destination: Grey Hack's terminal behind the batch, or VM output outside the game.
    screen: Rc<dyn Stdout>,
}

impl Terminal {
    pub(crate) fn send(&self) {
        if let Some(output) = &self.output {
            output.send();
        }
    }

    /// Displays `text` like player-typed terminal text, independently of the program batch.
    pub(crate) fn show(&self, text: &str) {
        self.screen.print(text, PrintOptions::default());
    }
}

impl FromVm for Terminal {
    fn from_vm(vm: &Vm) -> Self {
        let output = vm.host_data::<Game>().map(|game| Rc::clone(&game.output));
        let screen = output.as_ref().map_or_else(
            || Rc::clone(vm.stdout()),
            |output| Rc::clone(&output.stdout),
        );

        Self { output, screen }
    }
}

/// Flushes the program batch to its terminal ([`Terminal::send`]).
pub(crate) fn send_output(vm: &Vm) {
    Terminal::from_vm(vm).send();
}

#[cfg(test)]
#[path = "tests/terminal.rs"]
mod tests;
