use greybel_io::{PrintOptions, TextScreen};

/// Prints `text` with a trailing newline, replacing the current line when `replace` is true.
pub fn print(screen: &mut TextScreen, text: &str, replace: bool) {
    screen.print(
        text,
        PrintOptions {
            new_line: true,
            replace,
        },
    );
}
