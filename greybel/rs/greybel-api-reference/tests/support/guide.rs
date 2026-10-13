//! The page's guide in `greybel/ts/greybel-api-reference-web/guide`: what `guide.json` lists.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// What `guide.json` lists: its introduction's Markdown, and each chapter's anchor and Markdown.
pub struct Guide {
    pub introduction: Option<String>,
    pub chapters: Vec<(String, String)>,
}

impl Guide {
    pub fn read() -> Self {
        let file = folder().join("guide.json");
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        let guide: Value = serde_json::from_str(&text).expect("guide.json is JSON");

        assert!(guide["title"].is_string(), "guide.json has a title");

        Self {
            introduction: guide["introduction"].as_str().map(read),
            chapters: guide["chapters"]
                .as_array()
                .expect("chapters is a list")
                .iter()
                .map(|file| {
                    let file = file.as_str().expect("a chapter is a file name");

                    (anchor(file), read(file))
                })
                .collect(),
        }
    }

    /// The introduction's Markdown, then each chapter's.
    pub fn markdown(&self) -> impl Iterator<Item = &String> {
        self.introduction
            .iter()
            .chain(self.chapters.iter().map(|(_, markdown)| markdown))
    }
}

fn folder() -> PathBuf {
    Path::new(env!("GREYBEL_TS")).join("greybel-api-reference-web/guide")
}

fn read(file: &str) -> String {
    let path = folder().join(file);

    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The anchor of a chapter: `guide-` and its file's name.
fn anchor(file: &str) -> String {
    let name = Path::new(file).file_stem().expect("a file name");

    format!("guide-{}", name.to_string_lossy())
}
