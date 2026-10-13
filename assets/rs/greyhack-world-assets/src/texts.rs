use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTextTable {
    locale: String,
    content: HashMap<String, String>,
}

impl GameTextTable {
    const BYTE_ORDER_MARK: char = '\u{feff}';

    #[must_use]
    pub fn english() -> &'static Self {
        static ENGLISH: LazyLock<GameTextTable> = LazyLock::new(|| {
            GameTextTable::parse(crate::bundle::text("texts/en.json"))
                .expect("the bundled game texts are valid")
        });

        &ENGLISH
    }

    pub fn parse(json: &str) -> serde_json::Result<Self> {
        #[derive(serde::Deserialize)]
        struct File {
            locale: String,
            content: HashMap<String, String>,
        }

        let file: File = serde_json::from_str(json.trim_start_matches(Self::BYTE_ORDER_MARK))?;

        Ok(Self {
            locale: file.locale,
            content: file.content,
        })
    }

    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.content.get(key).map(String::as_str)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.content.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}
