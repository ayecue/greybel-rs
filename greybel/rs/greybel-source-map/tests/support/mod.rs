use greybel_source_map::Location;

/// Creates an optional source location for line `line` of `source`.
pub fn at(source: &str, line: u32) -> Location<'_> {
    Location::new(source, line)
}
