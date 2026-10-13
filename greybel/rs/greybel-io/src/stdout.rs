use crate::types::PrintOptions;

impl PrintOptions {
    pub const OPEN_LINE: Self = Self {
        new_line: false,
        replace: false,
    };
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            new_line: true,
            replace: false,
        }
    }
}

pub trait Stdout {
    fn print(&self, text: &str, options: PrintOptions);

    fn clear(&self);

    fn flush(&self) {}

    fn flush_if_due(&self) {}
}
