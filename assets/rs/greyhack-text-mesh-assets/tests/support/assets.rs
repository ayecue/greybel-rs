use greyhack_text_mesh_assets::{TERMINAL_THEME, TEXT_ASSETS};
use text_mesh_core::Run;

pub fn parse(text: &str) -> Vec<Run> {
    TERMINAL_THEME.component(&TEXT_ASSETS).parse(text).runs
}
