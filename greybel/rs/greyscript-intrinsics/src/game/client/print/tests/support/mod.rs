use std::cell::RefCell;

use greybel_interpreter::{PrintOptions, Stdout};

/// Output texts printed by a `PrintLine` renderer test.
#[derive(Default)]
pub(super) struct Lines(pub(super) RefCell<Vec<String>>);

impl Stdout for Lines {
    fn print(&self, text: &str, _options: PrintOptions) {
        self.0.borrow_mut().push(text.to_owned());
    }

    fn clear(&self) {}
}
