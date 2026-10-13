use serde::Deserialize;
use text_mesh_core::Alignment;
use text_mesh_layout::{Element, ElementKind, Line, TerminalItem};
use text_mesh_test_support::reference::{Captures, Differences, KnownDifference, WIDTH};

pub const TOLERANCE: f32 = 0.01;

const CSPACE_END: &str = "TMP's </cspace> (TMP_Text.cs:7915-7925) also runs while \
    CalculatePreferredValues measures the text after GenerateTextMesh, overwriting the \
    xAdvance of the character before it with the measuring pass's unaligned m_xAdvance; \
    the layout retains the aligned value GenerateTextMesh stored";

pub const KNOWN_DIFFERENCES: &[KnownDifference] = &[
    KnownDifference {
        case: 4,
        field: "items[0].characters[5].advance",
        reason: CSPACE_END,
    },
    KnownDifference {
        case: 4,
        field: "items[0].characters[11].advance",
        reason: CSPACE_END,
    },
];

#[derive(Deserialize)]
pub struct Case {
    pub source: String,
    pub items: Vec<Item>,
}

#[derive(Deserialize)]
pub struct Item {
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub characters: Vec<CharacterRecord>,
    pub lines: Vec<LineRecord>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRecord {
    pub unicode: u32,
    pub sprite: bool,
    pub visible: bool,
    pub line: usize,
    pub origin: f32,
    pub advance: f32,
    pub baseline: f32,
    pub ascender: f32,
    pub descender: f32,
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub point_size: f32,
    pub scale: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineRecord {
    pub first: usize,
    pub last: usize,
    pub ascender: f32,
    pub descender: f32,
    pub baseline: f32,
    pub line_height: f32,
    pub width: f32,
    pub length: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub alignment: u32,
}

/// TMP's layout of every test case (`text_mesh_test_support::cases`), as the reference project captures it with the test
/// setup; refreshed first with `REFRESH_REFERENCE` set.
pub fn captures() -> Captures<Case> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures/tmp-layout.json");

    text_mesh_test_support::refresh_reference("layout", |reference, setup| {
        reference
            .capture(setup, &text_mesh_test_support::cases(), WIDTH)?
            .write_layout(&path)
    });

    Captures::read(&path)
}

pub fn lay_out(source: &str, width: f32) -> Vec<TerminalItem> {
    text_mesh_test_support::setup()
        .terminal_layout()
        .layout(source, width)
}

/// Records where the layout differs from TMP's.
pub fn compare(differences: &mut Differences, expected: &[Item], actual: &[TerminalItem]) {
    Comparison(differences).items(expected, actual);
}

struct Comparison<'d>(&'d mut Differences);

impl Comparison<'_> {
    fn items(&mut self, expected: &[Item], actual: &[TerminalItem]) {
        self.count("items", expected.len(), actual.len());
        for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
            let field = format!("items[{index}]");

            self.float(&format!("{field}.top"), expected.top, actual.top);
            self.float(
                &format!("{field}.width"),
                expected.width,
                actual.layout.width,
            );
            self.float(
                &format!("{field}.height"),
                expected.height,
                actual.layout.height,
            );

            let layout = &actual.layout;

            self.count(
                &format!("{field}.characters"),
                expected.characters.len(),
                layout.elements.len(),
            );
            for (position, (expected, actual)) in
                expected.characters.iter().zip(&layout.elements).enumerate()
            {
                self.character(&format!("{field}.characters[{position}]"), expected, actual);
            }

            self.count(
                &format!("{field}.lines"),
                expected.lines.len(),
                layout.lines.len(),
            );
            for (position, (expected, actual)) in
                expected.lines.iter().zip(&layout.lines).enumerate()
            {
                self.line(&format!("{field}.lines[{position}]"), expected, actual);
            }
        }
    }

    fn character(&mut self, field: &str, expected: &CharacterRecord, actual: &Element) {
        let (unicode, sprite) = match actual.kind {
            ElementKind::Glyph { character, .. } => (character as u32, false),
            ElementKind::Sprite { index, .. } => (0xE000 + index, true),
        };

        self.count(
            &format!("{field}.unicode"),
            expected.unicode as usize,
            unicode as usize,
        );
        self.flag(&format!("{field}.sprite"), expected.sprite, sprite);
        self.flag(
            &format!("{field}.visible"),
            expected.visible,
            actual.visible,
        );
        self.count(&format!("{field}.line"), expected.line, actual.line);

        let floats = [
            ("origin", expected.origin, actual.origin),
            ("advance", expected.advance, actual.advance),
            ("baseline", expected.baseline, actual.baseline),
            ("ascender", expected.ascender, actual.ascender),
            ("descender", expected.descender, actual.descender),
            ("left", expected.left, actual.left),
            ("top", expected.top, actual.top),
            ("right", expected.right, actual.right),
            ("bottom", expected.bottom, actual.bottom),
            ("pointSize", expected.point_size, actual.point_size),
            ("scale", expected.scale, actual.scale),
        ];

        for (name, expected, actual) in floats {
            self.float(&format!("{field}.{name}"), expected, actual);
        }
    }

    fn line(&mut self, field: &str, expected: &LineRecord, actual: &Line) {
        self.count(&format!("{field}.first"), expected.first, actual.first);
        self.count(&format!("{field}.last"), expected.last, actual.last);

        let floats = [
            ("ascender", expected.ascender, actual.ascender),
            ("descender", expected.descender, actual.descender),
            ("baseline", expected.baseline, actual.baseline),
            ("lineHeight", expected.line_height, actual.line_height),
            ("width", expected.width, actual.width),
            ("length", expected.length, actual.length),
            ("marginLeft", expected.margin_left, actual.margin_left),
            ("marginRight", expected.margin_right, actual.margin_right),
        ];

        for (name, expected, actual) in floats {
            self.float(&format!("{field}.{name}"), expected, actual);
        }

        let alignment = match actual.alignment {
            Alignment::Left => 1,
            Alignment::Center => 2,
            Alignment::Right => 4,
            Alignment::Justified => 8,
            Alignment::Flush => 16,
        };

        self.count(
            &format!("{field}.alignment"),
            expected.alignment as usize,
            alignment,
        );
    }

    fn float(&mut self, field: &str, expected: f32, actual: f32) {
        self.0.float(field, expected, actual, TOLERANCE);
    }

    fn count(&mut self, field: &str, expected: usize, actual: usize) {
        self.0.count(field, expected, actual);
    }

    fn flag(&mut self, field: &str, expected: bool, actual: bool) {
        self.0.flag(field, expected, actual);
    }
}
