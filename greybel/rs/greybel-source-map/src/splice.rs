use crate::{Mapping, SourceMap};

impl SourceMap {
    /// Replaces `output_line` with the `lines` lines mapped by `other`. Requires `lines >= 1` and shifts subsequent
    /// generated lines down by `lines - 1`.
    pub fn splice(&mut self, output_line: u32, other: &Self, lines: u32) {
        let offset = lines.saturating_sub(1);
        let mut mappings = Vec::with_capacity(self.mappings.len() + other.mappings.len() + 1);

        for mapping in &self.mappings {
            mappings.extend(mapping.before(output_line));
            mappings.extend(
                mapping
                    .after(output_line)
                    .map(|after| after.down_by(offset)),
            );
        }

        for mapping in other
            .mappings
            .iter()
            .filter_map(|mapping| mapping.before(lines + 1))
        {
            let source = self.source_index(&other.sources[mapping.source as usize]);

            mappings.push(Mapping {
                source,
                ..mapping.down_by(output_line - 1)
            });
        }

        mappings.sort_by_key(|mapping| mapping.first);
        self.mappings = mappings;
    }
}
