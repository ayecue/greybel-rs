use std::io::{self, BufRead, Write};

use greybel_runtime::Controller;
use greybel_runtime::protocol::{StepMode, StopNotice, StopReason};

use super::TerminalHost;
use crate::report;

const HELP: &str = "\
continue, c, exit   run to the next breakpoint or `debugger` statement
next, n             run to the next statement of this function
step, s             run to the next statement, entering calls
out, o              run until this function returns
stack, bt           show the call stack
vars, v             show the variables of the current frame
terminate, q        end the program
help, h             show this help
anything else       run as code in the current frame, showing an expression's value";

/// Interactive stopped-program debugger with inspection, evaluation, resume, step, and terminate commands.
pub struct Debugger<'c> {
    controller: &'c Controller<TerminalHost>,
}

impl<'c> Debugger<'c> {
    pub fn new(controller: &'c Controller<TerminalHost>) -> Self {
        Self { controller }
    }

    /// Processes commands for the `notice` stop until continuing, stepping, or terminating the program.
    pub async fn serve(&self, notice: &StopNotice) {
        let place = notice
            .location
            .as_ref()
            .map_or_else(String::new, |location| {
                report::place(&location.uri, location.line, location.character)
            });

        println!(
            "Stopped ({}) at {place}. Enter `help` for the commands.",
            reason(notice.reason)
        );

        loop {
            let Some(line) = read_command(&place) else {
                self.controller.terminate();

                return;
            };

            match line.as_str() {
                "" => {}
                "continue" | "c" | "exit" => return self.controller.resume(),
                "next" | "n" => return self.controller.step(StepMode::Over),
                "step" | "s" => return self.controller.step(StepMode::Into),
                "out" | "o" => return self.controller.step(StepMode::Out),
                "terminate" | "q" => return self.controller.terminate(),
                "stack" | "bt" => self.show_stack().await,
                "vars" | "v" => self.show_variables().await,
                "help" | "h" => println!("{HELP}"),
                code => self.evaluate(code).await,
            }
        }
    }

    async fn show_stack(&self) {
        match self.controller.stack_trace().await {
            Ok(frames) => {
                for frame in frames {
                    let place = frame
                        .location
                        .as_ref()
                        .map_or_else(String::new, |location| {
                            report::place(&location.uri, location.line, location.character)
                        });

                    println!("  {} {place}", frame.name);
                }
            }
            Err(failure) => println!("{failure:?}"),
        }
    }

    async fn show_variables(&self) {
        let frames = match self.controller.stack_trace().await {
            Ok(frames) => frames,
            Err(failure) => return println!("{failure:?}"),
        };
        let Some(frame) = frames.first() else {
            return;
        };
        let scopes = match self.controller.scopes(frame.id).await {
            Ok(scopes) => scopes,
            Err(failure) => return println!("{failure:?}"),
        };

        for scope in scopes {
            println!("{}:", scope.name);

            match self.controller.variables(scope.reference).await {
                Ok(variables) => {
                    for variable in variables {
                        println!("  {} = {}", variable.name, variable.value);
                    }
                }
                Err(failure) => println!("  {failure:?}"),
            }
        }
    }

    async fn evaluate(&self, code: &str) {
        match self.controller.evaluate(code.to_owned()).await {
            Ok(Some(evaluation)) => println!("{}", evaluation.value),
            Ok(None) => {}
            Err(failure) => println!("{failure}"),
        }
    }
}

/// Prompts at `place` and reads a debugger command; returns `None` on EOF or input failure.
fn read_command(place: &str) -> Option<String> {
    print!("[{place}] > ");

    let _ = io::stdout().flush();
    let mut line = String::new();

    match io::stdin().lock().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim().to_owned()),
    }
}

fn reason(reason: StopReason) -> &'static str {
    match reason {
        StopReason::Entry => "entry",
        StopReason::Breakpoint => "breakpoint",
        StopReason::Debugger => "debugger statement",
        StopReason::Step => "step",
        StopReason::Pause => "pause",
    }
}
