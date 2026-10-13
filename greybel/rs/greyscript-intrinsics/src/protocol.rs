//! World-protocol operations sent by game intrinsics to the game server.

duplex_core::include_protocol!("$PROTOCOLS/world", major = 1, role = client);

impl From<&SourceMap> for greybel_source_map::SourceMap {
    fn from(map: &SourceMap) -> Self {
        Self::new(
            map.sources.clone(),
            map.mappings
                .iter()
                .map(|mapping| greybel_source_map::Mapping {
                    first: mapping.first,
                    last: mapping.last,
                    source: mapping.source,
                    line: mapping.line,
                })
                .collect(),
        )
    }
}

impl From<&greybel_source_map::SourceMap> for SourceMap {
    fn from(map: &greybel_source_map::SourceMap) -> Self {
        Self {
            sources: map.sources().to_vec(),
            mappings: map
                .mappings()
                .iter()
                .map(|mapping| SourceMapping {
                    first: mapping.first,
                    last: mapping.last,
                    source: mapping.source,
                    line: mapping.line,
                })
                .collect(),
        }
    }
}
