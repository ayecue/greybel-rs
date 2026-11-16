use crate::hash::ValueHash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub(crate) const HIGHLIGHT: Self = Self::new(255, 255, 0, 64);
    const NAMED_COLORS: [(i32, Self); 10] = [
        (ValueHash::of("red"), Self::new(255, 0, 0, 255)),
        (ValueHash::of("lightblue"), Self::new(173, 216, 230, 255)),
        (ValueHash::of("blue"), Self::new(0, 0, 255, 255)),
        (ValueHash::of("grey"), Self::new(128, 128, 128, 255)),
        (ValueHash::of("black"), Self::BLACK),
        (ValueHash::of("green"), Self::new(0, 255, 0, 255)),
        (ValueHash::of("white"), Self::WHITE),
        (ValueHash::of("orange"), Self::new(255, 128, 0, 255)),
        (ValueHash::of("purple"), Self::new(160, 32, 240, 255)),
        (ValueHash::of("yellow"), Self::new(255, 235, 4, 255)),
    ];
    pub const WHITE: Self = Self::new(255, 255, 255, 255);

    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub fn blend(self, background: Self) -> Self {
        let alpha = u32::from(self.a);
        let channel = |front: u8, back: u8| {
            let blended_value =
                (u32::from(front) * alpha + u32::from(back) * (255 - alpha) + 127) / 255;

            u8::try_from(blended_value).unwrap_or(u8::MAX)
        };

        Self::new(
            channel(self.r, background.r),
            channel(self.g, background.g),
            channel(self.b, background.b),
            255,
        )
    }

    /// The color a `<color=...>`, `<mark=...>` or `<#...>` tag value names: `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`
    /// or a color name such as `red`. Hex digits are case-insensitive; names are lowercase.
    #[must_use]
    pub fn from_tag_value(value: &str) -> Option<Self> {
        let Some(digits) = value.strip_prefix('#') else {
            return Self::from_name_hash(ValueHash::of(value));
        };

        if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }

        let units: Vec<u16> = value.encode_utf16().collect();

        matches!(units.len(), 4 | 5 | 7 | 9).then(|| Self::from_tag(&units, units.len()))
    }

    pub(crate) fn from_name_hash(value: i32) -> Option<Self> {
        Self::NAMED_COLORS
            .iter()
            .find(|(hash, _)| *hash == value)
            .map(|&(_, color)| color)
    }

    pub(crate) fn from_tag(text: &[u16], length: usize) -> Self {
        let short = |index: usize| Hex::byte(text[index], text[index]);
        let long = |index: usize| Hex::byte(text[index], text[index + 1]);

        match length {
            4 => Self::new(short(1), short(2), short(3), 255),
            5 => Self::new(short(1), short(2), short(3), short(4)),
            7 => Self::new(long(1), long(3), long(5), 255),
            9 => Self::new(long(1), long(3), long(5), long(7)),
            10 => Self::new(short(7), short(8), short(9), 255),
            11 => Self::new(short(7), short(8), short(9), short(10)),
            13 => Self::new(long(7), long(9), long(11), 255),
            15 => Self::new(long(7), long(9), long(11), long(13)),
            _ => Self::WHITE,
        }
    }

    pub(crate) fn from_value(text: &[u16], start: usize, length: usize) -> Self {
        let long = |index: usize| Hex::byte(text[start + index], text[start + index + 1]);

        match length {
            7 => Self::new(long(1), long(3), long(5), 255),
            9 => Self::new(long(1), long(3), long(5), long(7)),
            _ => Self::WHITE,
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(crate) fn multiply(self, other: Self) -> Self {
        let channel = |a: u8, b: u8| (f32::from(a) / 255.0 * (f32::from(b) / 255.0) * 255.0) as u8;

        Self::new(
            channel(self.r, other.r),
            channel(self.g, other.g),
            channel(self.b, other.b),
            channel(self.a, other.a),
        )
    }

    pub(crate) fn with_alpha_at_most(self, alpha: u8) -> Self {
        Self {
            a: self.a.min(alpha),
            ..self
        }
    }
}

pub(crate) struct Hex;

impl Hex {
    pub(crate) fn digit(unit: u16) -> u8 {
        match unit {
            0x30..=0x39 => (unit - 0x30) as u8,
            0x41..=0x46 => (unit - 0x41 + 10) as u8,
            0x61..=0x66 => (unit - 0x61 + 10) as u8,
            _ => 15,
        }
    }

    pub(crate) fn byte(high: u16, low: u16) -> u8 {
        Self::digit(high) * 16 + Self::digit(low)
    }
}
