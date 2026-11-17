//! Provider-side terminal-view bindings for page drawing.

duplex_core::include_protocol!("$PROTOCOLS/terminal", major = 1, role = provider);

impl LineLimits {
    /// The split_rule these limits describe.
    #[must_use]
    pub fn split_rule(&self) -> text_mesh_core::LineSplitRule {
        text_mesh_core::LineSplitRule::Limits(text_mesh_core::LineLimits::new(
            self.text_per_line as usize,
            self.tags_per_line.map(|limit| limit as usize),
        ))
    }
}
