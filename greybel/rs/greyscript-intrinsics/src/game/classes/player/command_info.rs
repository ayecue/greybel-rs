use greybel_interpreter::{Value, native};
use greyhack_world_assets::GameTextTable;

use crate::error::IntrinsicError;

/// Manual sections joined by `command_info` (`TranslationManager.GetManualExtraPrefixContent`).
const SECTIONS: [&str; 3] = ["USAGE", "DETAIL", "HELP"];
const USAGE_SUFFIX: &str = "_USAGE";

/// Implements `command_info` using English manual text.
/// Uppercases the key; command results join usage, details, and help with blank lines, while unknown keys return themselves (`PlayerIntrinsics.BuildManualCommandInfo`).
#[native]
pub fn command_info(#[name = "keyCommand"] key: &Value) -> Result<String, IntrinsicError> {
    let key = key
        .as_str()
        .filter(|key| !key.is_empty())
        .ok_or(IntrinsicError::InvalidArguments("command_info"))?
        .to_uppercase();
    let texts = GameTextTable::english();
    let text = |key: &str| texts.get(key).map_or_else(|| key.to_owned(), str::to_owned);

    if key.ends_with(USAGE_SUFFIX) {
        return Ok(text(&key));
    }

    let sections: Vec<&str> = SECTIONS
        .iter()
        .filter_map(|section| texts.get(&format!("{key}_{section}")))
        .filter(|section| !section.trim().is_empty())
        .collect();

    Ok(if sections.is_empty() {
        text(&key)
    } else {
        sections.join("\n\n")
    })
}
