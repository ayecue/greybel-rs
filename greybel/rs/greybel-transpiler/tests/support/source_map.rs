use greybel_source_map::Location;
use greybel_transpiler::Output;

/// The file and line the built line with `text` came from.
pub fn origin<'o>(output: &'o Output, text: &str) -> Option<Location<'o>> {
    let line = output
        .code
        .lines()
        .position(|line| line.contains(text))
        .unwrap_or_else(|| panic!("{text} in {}", output.code));

    output.source_map.locate(u32::try_from(line).unwrap() + 1)
}
