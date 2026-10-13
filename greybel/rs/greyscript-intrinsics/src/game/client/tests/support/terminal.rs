use std::cell::RefCell;
use std::rc::Rc;

use greybel_interpreter::{PrintOptions, Stdout};
use scheduler::ManualClock;

use super::super::TerminalOutput;

/// The texts the terminal was sent, one for each print.
#[derive(Default)]
pub struct SentTexts(pub RefCell<Vec<String>>);

impl Stdout for SentTexts {
    fn print(&self, text: &str, _options: PrintOptions) {
        self.0.borrow_mut().push(text.to_owned());
    }

    fn clear(&self) {}
}

pub fn output() -> (TerminalOutput, Rc<SentTexts>, ManualClock) {
    let sent = Rc::new(SentTexts::default());
    let clock = ManualClock::new();
    let output = TerminalOutput::new(Rc::clone(&sent) as Rc<dyn Stdout>, Rc::new(clock.clone()));

    (output, sent, clock)
}
