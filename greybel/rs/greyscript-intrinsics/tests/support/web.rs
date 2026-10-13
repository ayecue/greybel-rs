use std::cell::RefCell;

use greybel_interpreter::{PrintOptions, Stdout};

pub const SCRIPT: &str = r#"
shell = get_shell
print shell.launch("/bin/pwd")
shell.launch("/bin/ls", "/home")
print command_info("ls_usage")
"#;

/// The lines a program printed.
#[derive(Default)]
pub struct Lines(pub RefCell<Vec<String>>);

impl Stdout for Lines {
    fn print(&self, text: &str, _options: PrintOptions) {
        self.0.borrow_mut().push(text.to_owned());
    }

    fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}
