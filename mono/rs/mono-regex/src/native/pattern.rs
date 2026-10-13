use super::emit::emit;
use super::error::PatternError;
use super::groups::{GroupTable, SlotTable};
use super::options::Options;
use super::parser;
use crate::RegexOptions;

#[derive(Debug)]
pub struct Pattern {
    pub source: String,
    pub groups: GroupTable,
}

pub fn translate(pattern: &str, options: RegexOptions) -> Result<Pattern, PatternError> {
    let error = |reason| PatternError::new(pattern, reason);
    let options = Options::from(options);
    let slots = SlotTable::count(pattern, options).map_err(error)?;
    let node = parser::parse(pattern, options, &slots).map_err(error)?;
    let groups = GroupTable::new(&node, slots).map_err(error)?;
    let source = emit(&node, &groups, options.ignore_case).map_err(error)?;

    Ok(Pattern { source, groups })
}

#[cfg(test)]
#[path = "tests/pattern.rs"]
mod tests;
