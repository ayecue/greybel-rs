use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use greybel_io::{BufferedStdout, Output, PrintOptions, Screen, Stdout};
use scheduler::{Clock, Sleep};

#[derive(Default)]
pub struct BatchRecorder(RefCell<Vec<Vec<Output>>>);

impl Screen for BatchRecorder {
    fn show(&self, batch: &[Output]) {
        self.0.borrow_mut().push(batch.to_vec());
    }
}

#[derive(Clone, Default)]
pub struct Manual(Rc<Cell<Duration>>);

impl Manual {
    pub fn advance(&self, by: Duration) {
        self.0.set(self.0.get() + by);
    }
}

impl Clock for Manual {
    fn now(&self) -> Duration {
        self.0.get()
    }

    fn sleep(&self, _delay: Duration) -> Sleep {
        Sleep::ready()
    }
}

pub fn stdout() -> (BufferedStdout<BatchRecorder, Manual>, Manual) {
    let time = Manual::default();

    (
        BufferedStdout::new(BatchRecorder::default(), time.clone()),
        time,
    )
}

pub fn print(stdout: &impl Stdout, text: &str) {
    stdout.print(text, PrintOptions::default());
}

pub fn batches(stdout: &BufferedStdout<BatchRecorder, Manual>) -> Vec<Vec<Output>> {
    stdout.screen().0.borrow().clone()
}
