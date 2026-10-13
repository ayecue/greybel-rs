use std::cell::{Cell, Ref, RefCell};
use std::fmt;
use std::io::{self, Write};

use crossterm::Command;
use crossterm::cursor::{MoveTo, MoveToColumn, MoveToPreviousLine};
use crossterm::terminal::{Clear, ClearType};
use greybel_io::{LineCursor, Output, PrintOptions, Screen};

const NEW_LINE: char = '\n';

pub struct TerminalScreen<W: Write = io::Stdout> {
    writer: RefCell<W>,
    cursor: Cell<LineCursor>,
}

impl<W: Write> TerminalScreen<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: RefCell::new(writer),
            cursor: Cell::new(LineCursor::Blank),
        }
    }

    pub fn writer(&self) -> Ref<'_, W> {
        self.writer.borrow()
    }

    fn print(&self, out: &mut String, text: &str, options: PrintOptions) -> fmt::Result {
        if options.replace {
            match self.cursor.get() {
                LineCursor::Blank => {}
                LineCursor::InLine => erase_line(out)?,
                LineCursor::BelowLine => {
                    MoveToPreviousLine(1).write_ansi(out)?;
                    erase_line(out)?;
                }
            }
        }

        out.push_str(text);
        if options.new_line {
            out.push(NEW_LINE);
        }

        self.cursor.set(self.cursor.get().after(text, options));

        Ok(())
    }

    fn clear(&self, out: &mut String) -> fmt::Result {
        Clear(ClearType::All).write_ansi(out)?;
        MoveTo(0, 0).write_ansi(out)?;
        self.cursor.set(LineCursor::Blank);

        Ok(())
    }

    fn render(&self, batch: &[Output]) -> String {
        let mut out = String::new();

        for output in batch {
            let _ = match output {
                Output::Print { text, options } => self.print(&mut out, text, *options),
                Output::Clear => self.clear(&mut out),
            };
        }

        out
    }
}

fn erase_line(out: &mut String) -> fmt::Result {
    Clear(ClearType::CurrentLine).write_ansi(out)?;

    MoveToColumn(0).write_ansi(out)
}

impl Default for TerminalScreen {
    fn default() -> Self {
        Self::new(io::stdout())
    }
}

impl<W: Write> Screen for TerminalScreen<W> {
    fn show(&self, batch: &[Output]) {
        let out = self.render(batch);
        let mut writer = self.writer.borrow_mut();
        let _ = writer.write_all(out.as_bytes());
        let _ = writer.flush();
    }
}
