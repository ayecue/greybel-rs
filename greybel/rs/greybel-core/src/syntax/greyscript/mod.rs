mod import_code;

pub use import_code::{ImportCode, ImportCodeMeta, ImportCodeRule};

use crate::parser::KeywordStatement;

pub struct GreyScriptRuleTable;

impl GreyScriptRuleTable {
    pub const STATEMENTS: &'static [&'static dyn KeywordStatement] = &[&ImportCodeRule];
}
