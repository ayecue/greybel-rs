//! Test stdin/stdout and protocol `Output` constructors.

use std::cell::RefCell;
use std::rc::Rc;

use greybel_io::{Echo, InputFuture, Key, Output, PrintOptions, Stdin, Stdout};

/// Shared print lines in output order.
pub type PrintLog = Rc<RefCell<Vec<String>>>;

/// [`Stdout`] that appends prints to [`PrintLog`] and clears the log on `clear`.
pub struct CapturedStdout(pub PrintLog);

impl Stdout for CapturedStdout {
    fn print(&self, text: &str, _options: PrintOptions) {
        self.0.borrow_mut().push(text.to_owned());
    }

    fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

/// [`Stdout`] that records `(replace)`-prefixed prints and `(clear)` entries for progress-output snapshots.
#[derive(Default)]
pub struct MarkedStdout(PrintLog);

impl MarkedStdout {
    /// Joins recorded lines using newlines.
    #[must_use]
    pub fn text(&self) -> String {
        self.0.borrow().join("\n")
    }

    /// Reconstructs newline-joined terminal contents from recorded prints. Replacing prints discard earlier lines,
    /// matching Grey Hack's `TerminalListAdapter.AddText`; clears empty the terminal.
    #[must_use]
    pub fn terminal_text(&self) -> String {
        let log = self.0.borrow();
        let start = log
            .iter()
            .rposition(|line| line.starts_with(REPLACE_MARKER) || line == CLEAR_MARKER)
            .unwrap_or(0);
        let visible_lines = log[start..].iter().filter(|line| *line != CLEAR_MARKER);

        visible_lines
            .map(|line| line.strip_prefix(REPLACE_MARKER).unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

const REPLACE_MARKER: &str = "(replace) ";
const CLEAR_MARKER: &str = "(clear)";

impl Stdout for MarkedStdout {
    fn print(&self, text: &str, options: PrintOptions) {
        let marker = if options.replace { REPLACE_MARKER } else { "" };

        self.0.borrow_mut().push(format!("{marker}{text}"));
    }

    fn clear(&self) {
        self.0.borrow_mut().push(CLEAR_MARKER.to_owned());
    }
}

/// [`Stdin`] backed by preloaded lines. Line reads return `prompt + line` with `*` appended when masked; key
/// reads return Enter.
pub struct ScriptedStdin(RefCell<Vec<String>>);

impl ScriptedStdin {
    #[must_use]
    pub fn new(lines: &[&str]) -> Self {
        Self(RefCell::new(
            lines.iter().map(|line| (*line).to_owned()).collect(),
        ))
    }
}

impl Stdin for ScriptedStdin {
    fn read_line(&self, prompt: &str, echo: Echo) -> InputFuture<String> {
        let line = self.0.borrow_mut().remove(0);
        let mark = match echo {
            Echo::Visible => "",
            Echo::Masked => "*",
            Echo::Hidden => "-",
        };
        let line = format!("{prompt}{line}{mark}");

        Box::pin(std::future::ready(Ok(line)))
    }

    fn read_key(&self, _prompt: &str) -> InputFuture<Key> {
        Box::pin(std::future::ready(Ok(Key::Enter)))
    }
}

/// Combined [`Stdout`]/[`Stdin`] that logs ordered `print`, `clear`, `flush`, `flush if due`, and `read` calls.
/// Reads return an empty line or Enter.
#[derive(Clone, Default)]
pub struct Recorder(Rc<RefCell<Vec<String>>>);

impl Recorder {
    /// Returns calls in recording order.
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.0.borrow().clone()
    }

    fn record(&self, call: impl Into<String>) {
        self.0.borrow_mut().push(call.into());
    }
}

impl Stdout for Recorder {
    fn print(&self, text: &str, _options: PrintOptions) {
        self.record(format!("print {text}"));
    }

    fn clear(&self) {
        self.record("clear");
    }

    fn flush(&self) {
        self.record("flush");
    }

    fn flush_if_due(&self) {
        self.record("flush if due");
    }
}

impl Stdin for Recorder {
    fn read_line(&self, _prompt: &str, _echo: Echo) -> InputFuture<String> {
        self.record("read");

        Box::pin(std::future::ready(Ok(String::new())))
    }

    fn read_key(&self, _prompt: &str) -> InputFuture<Key> {
        self.record("read");

        Box::pin(std::future::ready(Ok(Key::Enter)))
    }
}

/// Creates `Output::Print` for `text` with explicit newline and replacement options.
#[must_use]
pub fn print(text: &str, new_line: bool, replace: bool) -> Output {
    Output::Print {
        text: text.to_owned(),
        options: PrintOptions { new_line, replace },
    }
}

/// Creates `Output::Print` for `text` using default options.
#[must_use]
pub fn line(text: &str) -> Output {
    Output::Print {
        text: text.to_owned(),
        options: PrintOptions::default(),
    }
}
