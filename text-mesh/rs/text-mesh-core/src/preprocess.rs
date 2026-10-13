use crate::assets::TextAssetTable;
use crate::color::Hex;
use crate::hash::TagHash;

const STYLE_DEPTH: usize = 8;

pub(crate) struct Preprocessor<'a> {
    assets: &'a TextAssetTable,
    characters: Vec<char>,
    depth: usize,
    styles: [Vec<i32>; STYLE_DEPTH],
}

impl<'a> Preprocessor<'a> {
    const BR: i32 = TagHash::of("BR");
    const NBSP: i32 = TagHash::of("NBSP");
    const SLASH_STYLE: i32 = TagHash::of("/STYLE");
    const STYLE: i32 = TagHash::of("STYLE");
    const ZWSP: i32 = TagHash::of("ZWSP");

    pub(crate) fn run(text: &str, assets: &'a TextAssetTable) -> Vec<char> {
        let source: Vec<char> = text.chars().take_while(|&unit| unit != '\0').collect();
        let mut preprocessor = Self {
            assets,
            characters: Vec::with_capacity(source.len()),
            depth: 0,
            styles: Default::default(),
        };

        preprocessor.copy(&source, false);

        preprocessor.characters
    }

    fn copy(&mut self, source: &[char], escapes: bool) {
        let mut index = 0;

        while index < source.len() {
            let mut unit = source[index];

            if escapes && unit == '\\' && index + 1 < source.len() {
                match source[index + 1] {
                    '\\' => index += 1,
                    'n' => {
                        unit = '\n';
                        index += 1;
                    }
                    'u' if index + 5 < source.len() => {
                        unit = Self::from_hex_digits(&source[index + 2..index + 6]);
                        index += 5;
                    }
                    'U' if index + 9 < source.len() => {
                        unit = Self::from_hex_digits(&source[index + 2..index + 10]);
                        index += 9;
                    }
                    _ => {}
                }
            }

            if unit == '<' {
                match TagHash::read(source, index + 1) {
                    Some(Self::BR) => {
                        self.characters.push('\n');
                        index += 4;
                        continue;
                    }
                    Some(Self::NBSP) => {
                        self.characters.push('\u{a0}');
                        index += 6;
                        continue;
                    }
                    Some(Self::ZWSP) => {
                        self.characters.push('\u{200b}');
                        index += 6;
                        continue;
                    }
                    Some(Self::STYLE) => {
                        if let Some(end) = self.open_style(source, index) {
                            index = end + 1;
                            continue;
                        }
                    }
                    Some(Self::SLASH_STYLE) => {
                        self.close_style();
                        index += 8;
                        continue;
                    }
                    _ => {}
                }
            }

            self.characters.push(unit);
            index += 1;
        }
    }

    fn open_style(&mut self, source: &[char], start: usize) -> Option<usize> {
        let mut hash = TagHash::default();
        let mut end = None;

        for (index, &unit) in source.iter().enumerate().skip(start + 7) {
            if unit == '"' {
                continue;
            }

            if unit == '>' {
                end = Some(index);
                break;
            }

            hash = hash.push(unit as u32);
        }

        let end = end?;
        let assets = self.assets;
        let style = assets.style(hash.0)?;

        self.styles.get_mut(self.depth + 1)?.push(hash.0);
        self.depth += 1;

        let opening_chars: Vec<char> = style.open_markup.chars().collect();

        self.copy(&opening_chars, true);
        self.depth -= 1;

        Some(end)
    }

    fn close_style(&mut self) {
        let hash = self
            .styles
            .get_mut(self.depth + 1)
            .and_then(Vec::pop)
            .unwrap_or(0);
        let assets = self.assets;
        let Some(style) = assets.style(hash) else {
            return;
        };

        self.depth += 1;

        let closing_chars: Vec<char> = style.close_markup.chars().collect();

        self.copy(&closing_chars, true);
        self.depth -= 1;
    }

    fn from_hex_digits(digits: &[char]) -> char {
        let value = digits.iter().fold(0_u32, |value, &digit| {
            (value << 4) + u32::from(Hex::digit(u16::try_from(u32::from(digit)).unwrap_or(0)))
        });

        char::from_u32(value).unwrap_or('\u{fffd}')
    }
}
