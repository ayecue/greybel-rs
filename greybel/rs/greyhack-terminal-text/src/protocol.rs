//! Provider-side renderer bindings for the terminal-text protocol.

duplex_core::include_protocol!("$PROTOCOLS/terminal-text", major = 1, role = provider);

use text_mesh_core::LineSplitRule;

impl LineLimits {
    /// The split_rule these limits describe.
    #[must_use]
    pub fn split_rule(&self) -> LineSplitRule {
        LineSplitRule::Limits(text_mesh_core::LineLimits::new(
            self.text_per_line as usize,
            self.tags_per_line.map(|limit| limit as usize),
        ))
    }
}
