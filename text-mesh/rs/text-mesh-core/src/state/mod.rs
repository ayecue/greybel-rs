mod stack;

use stack::{CountedStyle, FixedValueStack, FontStyleCountTable};

use crate::color::{Color, Hex};
use crate::hash::{NameHash, ValueHash};
use crate::markup::{Markup, Tag};
use crate::number::TagNumber;
use crate::scanner::{ScannedTag, Unit, ValueKind};
use crate::style::{Alignment, FontWeight, LayoutChange, Length, LineHeight, Style};
use crate::text::TextComponent;

const ALPHA_DIGIT_COUNT: usize = 3;
const ALPHA_DIGITS: usize = "<alpha=".len();
const SIZE_VALUE: usize = "size=".len();
const COLOR_VALUE: usize = "color=".len();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct FontStyleTable {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    highlight: bool,
    subscript: bool,
    superscript: bool,
    lowercase: bool,
    uppercase: bool,
    small_caps: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Outcome {
    NoOutput,
    Sprite { index: u32, color: Color },
    Position(Length),
    Space(f32),
    Layout(LayoutChange),
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Case {
    Upper,
    Lower,
    SmallCaps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct FontMaterial {
    font: Option<u16>,
    material: Option<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct TextState<'a> {
    component: TextComponent<'a>,
    font: Option<u16>,
    material: Option<u16>,
    references: FixedValueStack<FontMaterial>,
    no_parsing: bool,
    styles: FontStyleTable,
    counts: FontStyleCountTable,
    weight: FontWeight,
    weights: FixedValueStack<FontWeight>,
    color: Color,
    colors: FixedValueStack<Color>,
    underline: Color,
    underlines: FixedValueStack<Color>,
    strikethrough: Color,
    strikethroughs: FixedValueStack<Color>,
    highlights: FixedValueStack<Color>,
    size: f32,
    sizes: FixedValueStack<f32>,
    character_spacing: f32,
    mono_spacing: f32,
    vertical_offset: f32,
    rotation: f32,
    scale: f32,
}

impl<'a> TextState<'a> {
    const ALIGNMENTS: [(i32, Alignment); 5] = [
        (ValueHash::of("left"), Alignment::Left),
        (ValueHash::of("right"), Alignment::Right),
        (ValueHash::of("center"), Alignment::Center),
        (ValueHash::of("justified"), Alignment::Justified),
        (ValueHash::of("flush"), Alignment::Flush),
    ];
    const ANGLE: [i32; 2] = [NameHash::of("angle"), NameHash::of_upper("angle")];
    const ANIM: [i32; 2] = [NameHash::of("anim"), NameHash::of_upper("anim")];
    const COLOR: [i32; 2] = [NameHash::of("color"), NameHash::of_upper("color")];
    const COLOR_LOWERCASE: i32 = NameHash::of("color");
    const DEFAULT: [i32; 2] = [ValueHash::of("default"), ValueHash::of("Default")];
    const INDEX: [i32; 2] = [NameHash::of("index"), NameHash::of_upper("index")];
    const MARGIN_LEFT: i32 = NameHash::of("left");
    const MARGIN_SIDES: [i32; 2] = [NameHash::of("left"), NameHash::of("right")];
    const MARK: [i32; 2] = [NameHash::of("mark"), NameHash::of_upper("mark")];
    const MATERIAL: [i32; 2] = [NameHash::of("material"), NameHash::of_upper("material")];
    const MAX_SIZE: f32 = 60.0;
    const NAME: [i32; 2] = [NameHash::of("name"), NameHash::of_upper("name")];
    const ONLY_BOLD: FontStyleTable = FontStyleTable {
        bold: true,
        italic: false,
        underline: false,
        strikethrough: false,
        highlight: false,
        subscript: false,
        superscript: false,
        lowercase: false,
        uppercase: false,
        small_caps: false,
    };
    const PADDING: i32 = NameHash::of("padding");
    const SPRITE: [i32; 2] = [NameHash::of("sprite"), NameHash::of_upper("sprite")];
    const TINT: [i32; 2] = [NameHash::of("tint"), NameHash::of_upper("tint")];

    pub(crate) fn new(component: TextComponent<'a>) -> Self {
        let color = component.color;

        Self {
            component,
            font: None,
            material: None,
            references: FixedValueStack::with_capacity(16, FontMaterial::default()),
            no_parsing: false,
            styles: FontStyleTable::default(),
            counts: FontStyleCountTable::default(),
            weight: FontWeight::REGULAR,
            weights: FixedValueStack::with_capacity(8, FontWeight::REGULAR),
            color,
            colors: FixedValueStack::with_capacity(16, color),
            underline: color,
            underlines: FixedValueStack::with_capacity(16, color),
            strikethrough: color,
            strikethroughs: FixedValueStack::with_capacity(16, color),
            highlights: FixedValueStack::growable(color),
            size: component.size,
            sizes: FixedValueStack::with_capacity(16, component.size),
            character_spacing: 0.0,
            mono_spacing: 0.0,
            vertical_offset: 0.0,
            rotation: 0.0,
            scale: 1.0,
        }
    }

    pub(crate) fn case(&self) -> Option<Case> {
        if self.styles.uppercase {
            Some(Case::Upper)
        } else if self.styles.lowercase {
            Some(Case::Lower)
        } else if self.styles.small_caps {
            Some(Case::SmallCaps)
        } else {
            None
        }
    }

    pub(crate) fn style(&self, small_caps: bool) -> Style {
        Style {
            color: self.color.with_alpha_at_most(self.component.color.a),
            bold: self.styles.bold,
            weight: self.weight,
            italic: self.styles.italic,
            underline: self.styles.underline.then_some(self.underline),
            strikethrough: self.styles.strikethrough.then_some(self.strikethrough),
            highlight: self.styles.highlight.then(|| self.highlights.peek()),
            size: self.size,
            subscript: self.styles.subscript,
            superscript: self.styles.superscript,
            small_caps,
            character_spacing: self.character_spacing,
            mono_spacing: self.mono_spacing,
            vertical_offset: self.vertical_offset,
            rotation: self.rotation,
            scale: self.scale,
            font: self.font,
            material: self.material,
        }
    }

    pub(crate) fn sprite_color(&self, sprite: Color, tint: bool) -> Color {
        let vertex_color = self.color;
        let color = if tint {
            sprite.multiply(vertex_color)
        } else {
            sprite
        };
        let alpha = if color.a < self.component.color.a {
            color.a.min(vertex_color.a)
        } else {
            self.component.color.a
        };

        Color { a: alpha, ..color }
    }

    pub(crate) fn apply(&mut self, tag: &ScannedTag) -> Outcome {
        let markup = Markup::from_name(tag.name());

        if markup == Some(Markup::Close(Tag::NoParse)) {
            self.no_parsing = false;

            return Outcome::NoOutput;
        }

        if self.no_parsing {
            return Outcome::Invalid;
        }

        if tag.text[0] == u16::from(b'#') && matches!(tag.length, 4 | 5 | 7 | 9) {
            self.push_color(Color::from_tag(&tag.text, tag.length));

            return Outcome::NoOutput;
        }

        let is_accepted = match markup {
            None => false,
            Some(Markup::NoEffect) => true,
            Some(Markup::Open(Tag::Sprite)) => return self.sprite(tag),
            Some(Markup::Open(Tag::Position)) => return self.position(tag),
            Some(Markup::Open(Tag::Space)) => return self.space(tag),
            Some(Markup::Open(open)) if let Some(outcome) = self.layout(open, tag) => {
                return outcome;
            }
            Some(Markup::Close(close)) if let Some(change) = Self::layout_end(close) => {
                return Outcome::Layout(change);
            }
            Some(Markup::Open(open)) => self.open(open, tag),
            Some(Markup::Close(close)) => self.close(close),
        };

        if is_accepted {
            Outcome::NoOutput
        } else {
            Outcome::Invalid
        }
    }

    fn open(&mut self, tag: Tag, scanned_tag: &ScannedTag) -> bool {
        let number = || scanned_tag.number(0);
        let valid = |value: f32| value != TagNumber::INVALID;

        match tag {
            Tag::Bold => {
                self.styles.bold = true;
                self.counts.add(CountedStyle::Bold);
                self.weight = FontWeight::BOLD;

                true
            }
            Tag::Italic => {
                self.styles.italic = true;
                self.counts.add(CountedStyle::Italic);
                if Self::ANGLE.contains(&scanned_tag.attributes[1].name.0) {
                    #[allow(clippy::cast_possible_truncation)]
                    let angle = scanned_tag.number(1) as i32;

                    if !(-180..=180).contains(&angle) {
                        return false;
                    }
                }

                true
            }
            Tag::Strikethrough => {
                self.styles.strikethrough = true;
                self.counts.add(CountedStyle::Strikethrough);
                self.strikethrough = if Self::COLOR.contains(&scanned_tag.attributes[1].name.0) {
                    scanned_tag.color(1).with_alpha_at_most(self.color.a)
                } else {
                    self.color
                };
                self.strikethroughs.add(self.strikethrough);

                true
            }
            Tag::Underline => {
                self.styles.underline = true;
                self.counts.add(CountedStyle::Underline);
                self.underline = if Self::COLOR.contains(&scanned_tag.attributes[1].name.0) {
                    scanned_tag.color(1).with_alpha_at_most(self.color.a)
                } else {
                    self.color
                };
                self.underlines.add(self.underline);

                true
            }
            Tag::Mark => self.mark(scanned_tag),
            Tag::Subscript => {
                self.counts.add(CountedStyle::Subscript);
                self.styles.subscript = true;

                true
            }
            Tag::Superscript => {
                self.counts.add(CountedStyle::Superscript);
                self.styles.superscript = true;

                true
            }
            Tag::FontWeight => {
                let value = number();

                if !valid(value) {
                    return false;
                }

                #[allow(clippy::cast_possible_truncation)]
                match value as i32 {
                    100 => self.weight = FontWeight::THIN,
                    200 => self.weight = FontWeight::EXTRA_LIGHT,
                    300 => self.weight = FontWeight::LIGHT,
                    400 => self.weight = FontWeight::REGULAR,
                    500 => self.weight = FontWeight::MEDIUM,
                    600 => self.weight = FontWeight::SEMI_BOLD,
                    700 => self.weight = FontWeight::BOLD,
                    800 => self.weight = FontWeight::HEAVY,
                    900 => self.weight = FontWeight::BLACK,
                    _ => {}
                }

                self.weights.add(self.weight);

                true
            }
            Tag::Scale => {
                let value = number();

                if !valid(value) {
                    return false;
                }

                self.scale = value;
                self.rotation = 0.0;

                true
            }
            Tag::Rotate => {
                let value = number();

                if !valid(value) {
                    return false;
                }

                self.rotation = value;
                self.scale = 1.0;

                true
            }
            Tag::VerticalOffset => {
                let value = number();

                if !valid(value) {
                    return false;
                }

                match scanned_tag.unit {
                    Unit::Pixels => self.vertical_offset = value,
                    Unit::FontUnits => self.vertical_offset = value * self.size,
                    Unit::Percentage => return false,
                }

                true
            }
            Tag::Page | Tag::Link | Tag::Action => true,
            Tag::Size => self.size(scanned_tag),
            Tag::Font => self.font(scanned_tag),
            Tag::Material => self.material(scanned_tag),
            Tag::Gradient => false,
            Tag::Position | Tag::Space => false,
            Tag::Alpha => {
                if scanned_tag.attributes[0].length != ALPHA_DIGIT_COUNT {
                    return false;
                }

                self.color.a = Hex::byte(
                    scanned_tag.text[ALPHA_DIGITS],
                    scanned_tag.text[ALPHA_DIGITS + 1],
                );

                true
            }
            Tag::Color => self.color(scanned_tag),
            Tag::CharacterSpace => {
                let value = number();

                if !valid(value) {
                    return false;
                }

                match scanned_tag.unit {
                    Unit::Pixels => self.character_spacing = value,
                    Unit::FontUnits => self.character_spacing = value * self.size,
                    Unit::Percentage => return false,
                }

                true
            }
            Tag::Monospace => {
                let value = number();

                if !valid(value) {
                    return false;
                }

                match scanned_tag.unit {
                    Unit::Pixels => self.mono_spacing = value,
                    Unit::FontUnits => self.mono_spacing = value * self.size,
                    Unit::Percentage => return false,
                }

                true
            }
            Tag::Lowercase => {
                self.styles.lowercase = true;
                self.counts.add(CountedStyle::Lowercase);

                true
            }
            Tag::Uppercase => {
                self.styles.uppercase = true;
                self.counts.add(CountedStyle::Uppercase);

                true
            }
            Tag::SmallCaps => {
                self.styles.small_caps = true;

                true
            }
            Tag::NoParse => {
                self.no_parsing = true;

                true
            }
            Tag::Sprite
            | Tag::Indent
            | Tag::LineIndent
            | Tag::Margin
            | Tag::MarginLeft
            | Tag::MarginRight
            | Tag::LineHeight
            | Tag::Width
            | Tag::Align
            | Tag::NoBreak => false,
        }
    }

    fn layout(&self, tag: Tag, scanned_tag: &ScannedTag) -> Option<Outcome> {
        let length = |index: usize, unit: Unit| {
            let value = scanned_tag.number(index);

            (value != TagNumber::INVALID).then_some(match unit {
                Unit::Pixels => Length::Pixels(value),
                Unit::FontUnits => Length::Pixels(value * self.size),
                Unit::Percentage => Length::Percentage(value),
            })
        };
        let change = match tag {
            Tag::Indent => length(0, scanned_tag.unit).map(LayoutChange::Indent),
            Tag::LineIndent => length(0, scanned_tag.unit).map(LayoutChange::LineIndent),
            Tag::MarginLeft => length(0, scanned_tag.unit).map(|left| LayoutChange::Margin {
                left: Some(left),
                right: None,
            }),
            Tag::MarginRight => length(0, scanned_tag.unit).map(|right| LayoutChange::Margin {
                left: None,
                right: Some(right),
            }),
            Tag::Margin => match scanned_tag.attributes[0].kind {
                ValueKind::Number => length(0, scanned_tag.unit).map(|both| LayoutChange::Margin {
                    left: Some(both),
                    right: Some(both),
                }),
                ValueKind::None => {
                    let (mut left, mut right) = (None, None);

                    for (index, attribute) in scanned_tag.listed_attributes().skip(1) {
                        let side = attribute.name.0;

                        if !Self::MARGIN_SIDES.contains(&side) {
                            continue;
                        }

                        let value = length(index, attribute.unit)?;

                        if side == Self::MARGIN_LEFT {
                            left = Some(value);
                        } else {
                            right = Some(value);
                        }
                    }

                    Some(LayoutChange::Margin { left, right })
                }
                ValueKind::Color | ValueKind::String => None,
            },
            Tag::LineHeight => {
                let value = scanned_tag.number(0);

                (value != TagNumber::INVALID).then_some({
                    LayoutChange::LineHeight(match scanned_tag.unit {
                        Unit::Pixels => LineHeight::Pixels(value),
                        Unit::FontUnits => LineHeight::Pixels(value * self.size),
                        Unit::Percentage => LineHeight::Percentage {
                            value,
                            size: self.size,
                            font: self.font,
                        },
                    })
                })
            }
            Tag::Width if scanned_tag.unit != Unit::FontUnits => {
                length(0, scanned_tag.unit).map(LayoutChange::Width)
            }
            Tag::Width => None,
            Tag::Align => Self::ALIGNMENTS
                .iter()
                .find(|(hash, _)| *hash == scanned_tag.attributes[0].value.0)
                .map(|&(_, alignment)| LayoutChange::Align(alignment)),
            Tag::NoBreak => Some(LayoutChange::NoBreak(true)),
            _ => return None,
        };

        Some(change.map_or(Outcome::Invalid, Outcome::Layout))
    }

    fn layout_end(tag: Tag) -> Option<LayoutChange> {
        match tag {
            Tag::Indent => Some(LayoutChange::IndentEnd),
            Tag::LineIndent => Some(LayoutChange::LineIndentEnd),
            Tag::Margin => Some(LayoutChange::MarginEnd),
            Tag::LineHeight => Some(LayoutChange::LineHeightEnd),
            Tag::Width => Some(LayoutChange::WidthEnd),
            Tag::Align => Some(LayoutChange::AlignEnd),
            Tag::NoBreak => Some(LayoutChange::NoBreak(false)),
            _ => None,
        }
    }

    fn close(&mut self, tag: Tag) -> bool {
        match tag {
            Tag::Bold => {
                if self.counts.remove(CountedStyle::Bold) == 0 {
                    self.styles.bold = false;
                    self.weight = self.weights.peek();
                }
            }
            Tag::Italic => {
                if self.counts.remove(CountedStyle::Italic) == 0 {
                    self.styles.italic = false;
                }
            }
            Tag::Strikethrough => {
                if self.counts.remove(CountedStyle::Strikethrough) == 0 {
                    self.styles.strikethrough = false;
                }

                self.strikethrough = self.strikethroughs.remove();
            }
            Tag::Underline => {
                self.underline = self.underlines.remove();
                if self.counts.remove(CountedStyle::Underline) == 0 {
                    self.styles.underline = false;
                }

                self.underline = self.underlines.remove();
            }
            Tag::Mark => {
                self.highlights.remove();
                if self.counts.remove(CountedStyle::Highlight) == 0 {
                    self.styles.highlight = false;
                }
            }
            Tag::Subscript => {
                if self.styles.subscript && self.counts.remove(CountedStyle::Subscript) == 0 {
                    self.styles.subscript = false;
                }
            }
            Tag::Superscript => {
                if self.styles.superscript && self.counts.remove(CountedStyle::Superscript) == 0 {
                    self.styles.superscript = false;
                }
            }
            Tag::FontWeight => {
                self.weights.remove();
                self.weight = if self.styles == Self::ONLY_BOLD {
                    FontWeight::BOLD
                } else {
                    self.weights.peek()
                };
            }
            Tag::VerticalOffset => self.vertical_offset = 0.0,
            Tag::Font => {
                let reference = self.references.remove();

                self.font = reference.font;
                self.material = reference.material;
            }
            Tag::Material => self.material = self.references.remove().material,
            Tag::Size => self.size = self.sizes.remove(),
            Tag::Color => self.color = self.colors.remove(),
            Tag::CharacterSpace => self.character_spacing = 0.0,
            Tag::Monospace => self.mono_spacing = 0.0,
            Tag::Lowercase => {
                if self.counts.remove(CountedStyle::Lowercase) == 0 {
                    self.styles.lowercase = false;
                }
            }
            Tag::Uppercase => {
                if self.counts.remove(CountedStyle::Uppercase) == 0 {
                    self.styles.uppercase = false;
                }
            }
            Tag::SmallCaps => self.styles.small_caps = false,
            Tag::Position
            | Tag::NoBreak
            | Tag::Link
            | Tag::Align
            | Tag::Width
            | Tag::Gradient
            | Tag::Indent
            | Tag::LineIndent
            | Tag::Margin
            | Tag::LineHeight
            | Tag::Action => {}
            Tag::Scale | Tag::Rotate => {
                self.scale = 1.0;
                self.rotation = 0.0;
            }
            Tag::Page
            | Tag::Space
            | Tag::Alpha
            | Tag::Sprite
            | Tag::MarginLeft
            | Tag::MarginRight
            | Tag::NoParse => return false,
        }

        true
    }

    fn font(&mut self, tag: &ScannedTag) -> bool {
        if Self::DEFAULT.contains(&tag.attributes[0].value.0) {
            self.font = None;
            self.material = None;
            self.references.add(FontMaterial::default());

            return true;
        }

        let assets = self.component.assets;
        let Some(font) = assets.font(&tag.value_text(0)) else {
            return false;
        };
        let attribute = &tag.attributes[1];
        let material = if attribute.name.0 == 0 && attribute.value.0 == 0 {
            None
        } else if Self::MATERIAL.contains(&attribute.name.0) {
            match assets.material(&tag.value_text(1)) {
                Some(material) => Some(material),
                None => return false,
            }
        } else {
            return false;
        };

        self.font = Some(font);
        self.material = material;
        self.references.add(FontMaterial {
            font: self.font,
            material,
        });

        true
    }

    fn material(&mut self, tag: &ScannedTag) -> bool {
        if Self::DEFAULT.contains(&tag.attributes[0].value.0) {
            self.material = None;
            self.references.add(FontMaterial::default());

            return true;
        }

        let Some(material) = self.component.assets.material(&tag.value_text(0)) else {
            return false;
        };

        self.material = Some(material);
        self.references.add(FontMaterial {
            font: self.font,
            material: self.material,
        });

        true
    }

    fn position(&self, tag: &ScannedTag) -> Outcome {
        let value = tag.number(0);

        if value == TagNumber::INVALID {
            return Outcome::Invalid;
        }

        Outcome::Position(match tag.unit {
            Unit::Pixels => Length::Pixels(value),
            Unit::FontUnits => Length::Pixels(value * self.size),
            Unit::Percentage => Length::Percentage(value),
        })
    }

    fn space(&self, tag: &ScannedTag) -> Outcome {
        let value = tag.number(0);

        if value == TagNumber::INVALID {
            return Outcome::Invalid;
        }

        match tag.unit {
            Unit::Pixels => Outcome::Space(value),
            Unit::FontUnits => Outcome::Space(value * self.size),
            Unit::Percentage => Outcome::Invalid,
        }
    }

    fn mark(&mut self, tag: &ScannedTag) -> bool {
        self.styles.highlight = true;
        self.counts.add(CountedStyle::Highlight);

        let mut color = Color::HIGHLIGHT;

        for (index, attribute) in tag.listed_attributes() {
            let name = attribute.name.0;

            if Self::MARK.contains(&name) {
                if attribute.kind == ValueKind::Color {
                    color = tag.color(0);
                }
            } else if name == Self::COLOR_LOWERCASE {
                color = tag.color(index);
            } else if name == Self::PADDING && tag.parameters(index).len() != 4 {
                return false;
            }
        }

        self.highlights.add(color.with_alpha_at_most(self.color.a));

        true
    }

    fn size(&mut self, tag: &ScannedTag) -> bool {
        let value = tag.number(0);

        if value == TagNumber::INVALID {
            return false;
        }

        let base = self.component.size;
        let size = match tag.unit {
            Unit::Pixels if matches!(tag.text[SIZE_VALUE], 0x2B | 0x2D) => base + value,
            Unit::Pixels => value,
            Unit::FontUnits => base * value,
            Unit::Percentage => base * value / 100.0,
        };

        self.size = size.min(Self::MAX_SIZE);
        self.sizes.add(self.size);

        true
    }

    fn color(&mut self, tag: &ScannedTag) -> bool {
        if tag.text[COLOR_VALUE] == u16::from(b'#') && matches!(tag.length, 10 | 11 | 13 | 15) {
            self.push_color(Color::from_tag(&tag.text, tag.length));

            return true;
        }

        match Color::from_name_hash(tag.attributes[0].value.0) {
            Some(color) => {
                self.push_color(color);

                true
            }
            None => false,
        }
    }

    fn push_color(&mut self, color: Color) {
        self.color = color;
        self.colors.add(color);
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn sprite(&mut self, tag: &ScannedTag) -> Outcome {
        let assets = self.component.assets;
        let count = assets.sprite_count();
        let in_range = |value: f32| {
            let index = value as i32;

            (value != TagNumber::INVALID && index >= 0 && (index as u32) < count).then_some(index)
        };
        let first = &tag.attributes[0];

        if count == 0 || !matches!(first.kind, ValueKind::None | ValueKind::Number) {
            return Outcome::Invalid;
        }

        let mut index = -1;

        if first.kind == ValueKind::Number {
            match in_range(tag.number(0)) {
                Some(value) => index = value,
                None => return Outcome::Invalid,
            }
        }

        let mut color = Color::WHITE;
        let mut tint = false;

        for (position, attribute) in tag.listed_attributes() {
            let name = attribute.name.0;

            if Self::NAME.contains(&name) {
                match assets.sprite(attribute.value.0) {
                    Some(sprite_index) => index = i32::try_from(sprite_index).unwrap_or(-1),
                    None => return Outcome::Invalid,
                }
            } else if Self::INDEX.contains(&name) {
                match in_range(tag.number(1)) {
                    Some(value) => index = value,
                    None => return Outcome::Invalid,
                }
            } else if Self::TINT.contains(&name) {
                tint = tag.number(position) != 0.0;
            } else if Self::COLOR.contains(&name) {
                color = tag.color(position);
            } else if Self::ANIM.contains(&name) {
                let parameters = tag.parameters(position);

                if parameters.len() != 3 {
                    return Outcome::Invalid;
                }

                index = parameters[0] as i32;
            } else if !Self::SPRITE.contains(&name) {
                return Outcome::Invalid;
            }
        }

        if index < 0 {
            return Outcome::Invalid;
        }

        Outcome::Sprite {
            index: index as u32,
            color: self.sprite_color(color, tint),
        }
    }
}
