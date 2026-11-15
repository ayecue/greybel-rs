use crate::{Location, Mapping};

/// Associates generated lines with source files and lines. [`Self::sources`] contains file names;
/// [`Self::mappings`] contains ordered, non-overlapping mappings. Unmapped generated lines have no source location.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub(crate) sources: Vec<String>,
    pub(crate) mappings: Vec<Mapping>,
}

impl SourceMap {
    /// Creates a source map from `sources` and `mappings`, discarding invalid source or line ranges and any mapping
    /// that overlaps an earlier one.
    #[must_use]
    pub fn new(sources: Vec<String>, mut mappings: Vec<Mapping>) -> Self {
        let count = u32::try_from(sources.len()).unwrap_or(u32::MAX);

        mappings.retain(|mapping| mapping.source < count && mapping.first <= mapping.last);
        mappings.sort_by_key(|mapping| mapping.first);
        mappings.dedup_by(|next, previous| next.first <= previous.last);

        Self { sources, mappings }
    }

    /// Creates a source map from per-line locations. A `None` entry omits its generated line from the map.
    #[must_use]
    pub fn of_lines<'a>(locations: impl IntoIterator<Item = Option<Location<'a>>>) -> Self {
        let mut map = Self::default();

        for (output_line, location) in (1..).zip(locations) {
            let Some(Location { source, line }) = location else {
                continue;
            };
            let source = map.source_index(source);

            match map.mappings.last_mut() {
                Some(last) if last.continues_at(output_line, source, line) => {
                    last.last = output_line
                }
                _ => map
                    .mappings
                    .push(Mapping::single(output_line, source, line)),
            }
        }

        map
    }

    /// Creates an identity line mapping for `lines` generated lines in `source`.
    #[must_use]
    pub fn entire(source: &str, lines: u32) -> Self {
        Self::of_lines((1..=lines).map(|line| Some(Location::new(source, line))))
    }

    /// Applies `name` to every current source name.
    #[must_use]
    pub fn map_sources(mut self, mut name: impl FnMut(&str) -> String) -> Self {
        for source in &mut self.sources {
            *source = name(source);
        }

        self
    }

    #[must_use]
    pub fn sources(&self) -> &[String] {
        &self.sources
    }

    #[must_use]
    pub fn mappings(&self) -> &[Mapping] {
        &self.mappings
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Resolves `output_line` to a source location, or `None` when unmapped.
    #[must_use]
    pub fn locate(&self, output_line: u32) -> Option<Location<'_>> {
        let index = self
            .mappings
            .partition_point(|mapping| mapping.last < output_line);
        let mapping = self
            .mappings
            .get(index)
            .filter(|mapping| mapping.contains(output_line))?;

        Some(Location::new(
            &self.sources[mapping.source as usize],
            mapping.line_of(output_line),
        ))
    }

    /// Returns `source`'s index, inserting it when absent.
    pub(crate) fn source_index(&mut self, source: &str) -> u32 {
        let index = self
            .sources
            .iter()
            .position(|name| name == source)
            .unwrap_or_else(|| {
                self.sources.push(source.to_owned());

                self.sources.len() - 1
            });

        u32::try_from(index).expect("fewer than 2^32 sources")
    }
}
