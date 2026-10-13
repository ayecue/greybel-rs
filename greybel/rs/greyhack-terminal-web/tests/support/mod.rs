//! Shared helpers for `greyhack-terminal-web` integration tests.

pub mod meshes;
pub mod service;
pub mod ts;

use greyhack_terminal_web::TerminalView;

pub const WIDTH: f32 = 1000.0;
pub const HEIGHT: f32 = 300.0;

/// Returns `count` newline-separated red `line <index>` outputs.
pub fn output(count: usize) -> String {
    (0..count)
        .map(|index| format!("<color=#ff0000>line</color> {index}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn view_of(text: &str) -> TerminalView {
    let mut view = TerminalView::new(WIDTH, HEIGHT);

    view.set_text(text);

    view
}
