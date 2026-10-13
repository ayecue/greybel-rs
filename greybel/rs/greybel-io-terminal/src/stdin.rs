use std::future::ready;
use std::io::{self, BufRead, IsTerminal};
use std::rc::Rc;

use greybel_io::{Echo, InputFuture, Key, PrintOptions, Stdin, Stdout};

use crate::keyboard::{Keyboard, Press};

const INTERRUPT_EXIT_CODE: i32 = 130;
const MASK: &str = "*";
const UNMASK: &str = "\x08 \x08";

pub struct TerminalStdin {
    stdout: Rc<dyn Stdout>,
    keyboard: Keyboard,
}

impl TerminalStdin {
    pub fn new(stdout: Rc<dyn Stdout>) -> Self {
        Self {
            stdout,
            keyboard: Keyboard::default(),
        }
    }

    fn prompt(&self, prompt: &str) {
        if !prompt.is_empty() {
            self.stdout.print(prompt, PrintOptions::OPEN_LINE);
        }

        self.stdout.flush();
    }

    fn line(&self, echo: Echo) -> io::Result<String> {
        match echo {
            Echo::Masked | Echo::Hidden if io::stdin().is_terminal() => self.unechoed_line(echo),
            _ => read_line(),
        }
    }

    fn key(&self) -> io::Result<Key> {
        if !io::stdin().is_terminal() {
            return Ok(read_line()?.chars().next().map_or(Key::Enter, Key::Char));
        }

        let press = self.keyboard.raw()?.next()?;

        match press {
            Press::Key(key) => Ok(key),
            Press::Interrupt => self.interrupt(),
        }
    }

    /// Reads a raw-mode line. Masked `echo` displays `*` per typed character; hidden input displays nothing.
    fn unechoed_line(&self, echo: Echo) -> io::Result<String> {
        let masked = echo == Echo::Masked;
        let mut line = String::new();
        let raw = self.keyboard.raw()?;

        loop {
            match raw.next()? {
                Press::Key(Key::Enter) => break,
                Press::Key(Key::Char(char)) => {
                    line.push(char);
                    if masked {
                        self.echo(MASK);
                    }
                }
                Press::Key(Key::Backspace) => {
                    if line.pop().is_some() && masked {
                        self.echo(UNMASK);
                    }
                }
                Press::Key(_) => {}
                Press::Interrupt => {
                    drop(raw);
                    self.interrupt();
                }
            }
        }

        drop(raw);
        self.stdout.print("", PrintOptions::default());

        Ok(line)
    }

    fn echo(&self, text: &str) {
        self.stdout.print(text, PrintOptions::OPEN_LINE);
        self.stdout.flush();
    }

    fn interrupt(&self) -> ! {
        self.stdout.flush();

        std::process::exit(INTERRUPT_EXIT_CODE)
    }

    fn poll(&self) -> io::Result<Option<Key>> {
        if !io::stdin().is_terminal() {
            return Ok(None);
        }

        match self.keyboard.raw()?.poll()? {
            Some(Press::Key(key)) => Ok(Some(key)),
            Some(Press::Interrupt) => self.interrupt(),
            None => Ok(None),
        }
    }
}

fn read_line() -> io::Result<String> {
    let mut line = String::new();

    io::stdin().lock().read_line(&mut line)?;

    let len = line.trim_end_matches(['\r', '\n']).len();

    line.truncate(len);

    Ok(line)
}

impl Stdin for TerminalStdin {
    fn read_line(&self, prompt: &str, echo: Echo) -> InputFuture<String> {
        self.prompt(prompt);

        Box::pin(ready(self.line(echo)))
    }

    fn read_key(&self, prompt: &str) -> InputFuture<Key> {
        self.prompt(prompt);

        Box::pin(ready(self.key()))
    }

    fn try_read_key(&self, _prompt: &str) -> InputFuture<Option<Key>> {
        Box::pin(ready(self.poll()))
    }
}
