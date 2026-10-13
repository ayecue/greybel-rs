use std::sync::LazyLock;

use serde::Deserialize;

/// Item a generated computer can contain (`ItemsWorldGen.ItemWorld`): an image or a text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WorldItem {
    /// Image or text identifier (`ItemWorld.ID`): an image name, or a game text key starting with `CONV` for a
    /// conversation and `FILE` for a document.
    #[serde(rename = "ID")]
    pub id: String,
    /// File name, or empty when the file is named at random (`ItemWorld.name`).
    pub name: String,
    /// True if the item is only generated as a file, never as a mail (`ItemWorld.onlyFile`).
    #[serde(rename = "onlyFile")]
    pub only_file: bool,
    /// True if the item is only generated as a mail, never as a file (`ItemWorld.onlyMail`).
    #[serde(rename = "onlyMail")]
    pub only_mail: bool,
}

/// The items random content is drawn from (`ItemsWorldGen.ItemsWorld`), in the game's order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WorldItemTable {
    #[serde(rename = "itemImages")]
    pub images: Vec<WorldItem>,
    #[serde(rename = "itemTexts")]
    pub texts: Vec<WorldItem>,
    /// Texts of Neurobox networks (`ItemsWorld.itemTextsNB`).
    #[serde(rename = "itemTextsNB")]
    pub neurobox_texts: Vec<WorldItem>,
}

impl WorldItemTable {
    #[must_use]
    pub fn get() -> &'static Self {
        static ITEMS: LazyLock<WorldItemTable> = LazyLock::new(|| {
            serde_json::from_str(
                crate::bundle::text("config/world_items.json").trim_start_matches('\u{feff}'),
            )
            .expect("the bundled world items are valid")
        });

        &ITEMS
    }
}
