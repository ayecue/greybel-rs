use unicode_general_category::{GeneralCategory, get_general_category};

use crate::assets::TextAssetTable;
use crate::color::Color;
use crate::preprocess::Preprocessor;
use crate::scanner::ScannedTag;
use crate::state::{Case, Outcome, TextState};
use crate::style::{LayoutChange, Length, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextComponent<'a> {
    pub color: Color,
    pub size: f32,
    pub assets: &'a TextAssetTable,
}

impl<'a> TextComponent<'a> {
    #[must_use]
    pub const fn new(color: Color, size: f32, assets: &'a TextAssetTable) -> Self {
        Self {
            color,
            size,
            assets,
        }
    }

    #[must_use]
    pub const fn with_assets<'b>(&self, assets: &'b TextAssetTable) -> TextComponent<'b> {
        TextComponent {
            color: self.color,
            size: self.size,
            assets,
        }
    }

    #[must_use]
    pub fn parse(&self, text: &str) -> Text {
        let characters = Preprocessor::run(text, self.assets);
        let mut state = TextState::new(*self);
        let mut runs = RunBuilder::default();
        let mut index = 0;

        while index < characters.len() {
            let character = characters[index];

            if character == '<'
                && let Some(tag) = ScannedTag::scan(&characters, index + 1)
            {
                match state.apply(&tag) {
                    Outcome::NoOutput => {
                        index = tag.end + 1;
                        continue;
                    }
                    Outcome::Sprite {
                        index: sprite,
                        color,
                    } => {
                        runs.push_sprite(sprite, color, state.style(false));
                        index = tag.end + 1;
                        continue;
                    }
                    Outcome::Position(x) => {
                        runs.push(Run::Position { x });
                        index = tag.end + 1;
                        continue;
                    }
                    Outcome::Space(width) => {
                        runs.push(Run::Space { width });
                        index = tag.end + 1;
                        continue;
                    }
                    Outcome::Layout(change) => {
                        runs.push(Run::Layout(change));
                        index = tag.end + 1;
                        continue;
                    }
                    Outcome::Invalid => {}
                }
            }

            if character == '\n' {
                runs.push_break();
            } else {
                let (character, small_caps) = CaseMapping::apply(character, state.case());

                runs.push_char(character, state.style(small_caps));
            }

            index += 1;
        }

        Text {
            runs: runs.finish(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    Text {
        text: String,
        style: Style,
    },
    Sprite {
        index: u32,
        color: Color,
        style: Style,
    },
    Position {
        x: Length,
    },
    Space {
        width: f32,
    },
    Layout(LayoutChange),
    LineBreak,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Text {
    pub runs: Vec<Run>,
}

impl Text {
    #[must_use]
    pub fn plain(&self) -> String {
        self.runs
            .iter()
            .map(|run| match run {
                Run::Text { text, .. } => text.as_str(),
                Run::Sprite { .. } | Run::Position { .. } | Run::Space { .. } | Run::Layout(_) => {
                    ""
                }
                Run::LineBreak => "\n",
            })
            .collect()
    }
}

#[derive(Debug, Default)]
struct RunBuilder {
    runs: Vec<Run>,
}

impl RunBuilder {
    fn push_char(&mut self, character: char, style: Style) {
        if let Some(Run::Text { text, style: last }) = self.runs.last_mut()
            && *last == style
        {
            text.push(character);

            return;
        }

        self.runs.push(Run::Text {
            text: character.to_string(),
            style,
        });
    }

    fn push_sprite(&mut self, index: u32, color: Color, style: Style) {
        self.runs.push(Run::Sprite {
            index,
            color,
            style,
        });
    }

    fn push_break(&mut self) {
        self.runs.push(Run::LineBreak);
    }

    fn push(&mut self, run: Run) {
        self.runs.push(run);
    }

    fn finish(self) -> Vec<Run> {
        self.runs
    }
}

struct CaseMapping;

impl CaseMapping {
    fn apply(character: char, case: Option<Case>) -> (char, bool) {
        let Some(case) = case else {
            return (character, false);
        };
        let Some(character) = char::from_u32(character as u32 & 0xFFFF) else {
            return (character, false);
        };
        let category = get_general_category(character);

        match case {
            Case::Upper if category == GeneralCategory::LowercaseLetter => {
                (Self::single(character.to_uppercase(), character), false)
            }
            Case::Lower if category == GeneralCategory::UppercaseLetter => {
                (Self::single(character.to_lowercase(), character), false)
            }
            Case::SmallCaps if category == GeneralCategory::LowercaseLetter => {
                (Self::single(character.to_uppercase(), character), true)
            }
            _ => (character, false),
        }
    }

    fn single(mut chars: impl Iterator<Item = char>, original: char) -> char {
        match (chars.next(), chars.next()) {
            (Some(only), None) => only,
            _ => original,
        }
    }
}
