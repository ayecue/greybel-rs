const PLUS: u32 = b'+' as u32;
const MINUS: u32 = b'-' as u32;
const PERIOD: u32 = b'.' as u32;
const DIGIT_ZERO: u32 = b'0' as u32;
const DIGIT_NINE: u32 = b'9' as u32;
const HASH: u32 = b'#' as u32;
const QUOTE: u32 = b'"' as u32;
const PERCENT: u32 = b'%' as u32;
const EM: u32 = b'e' as u32;
const PIXELS: u32 = b'p' as u32;
const SPACE: u32 = b' ' as u32;
const LESS_THAN: u32 = b'<' as u32;
const GREATER_THAN: u32 = b'>' as u32;
const EQUALS: u32 = b'=' as u32;

mod attribute;

pub(crate) use attribute::{Attribute, Unit, ValueKind};

use crate::color::Color;
use crate::number::TagNumber;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Name,
    Value,
    Between,
}

#[derive(Debug, Clone)]
pub(crate) struct ScannedTag {
    pub(crate) text: [u16; Self::TEXT_CAPACITY],
    pub(crate) length: usize,
    pub(crate) attributes: [Attribute; Self::ATTRIBUTE_CAPACITY],
    pub(crate) unit: Unit,
    pub(crate) end: usize,
}

impl ScannedTag {
    const ATTRIBUTE_CAPACITY: usize = 8;
    const TEXT_CAPACITY: usize = 128;

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn scan(characters: &[char], start: usize) -> Option<Self> {
        let mut tag = Self {
            text: [0; Self::TEXT_CAPACITY],
            length: 0,
            attributes: [Attribute::default(); Self::ATTRIBUTE_CAPACITY],
            unit: Unit::Pixels,
            end: start,
        };
        let mut phase = Phase::Name;
        let mut kind = ValueKind::None;
        let mut current = 0;
        let mut spaced = false;
        let mut closed = false;

        for (index, &character) in characters.iter().enumerate().skip(start) {
            let code = character as u32;

            if code == 0 || tag.length >= Self::TEXT_CAPACITY || code == LESS_THAN {
                break;
            }

            if code == GREATER_THAN {
                closed = true;
                tag.end = index;
                break;
            }

            tag.text[tag.length] = code as u16;
            tag.length += 1;
            if phase == Phase::Value {
                let attribute = &mut tag.attributes[current];

                match kind {
                    ValueKind::None => {
                        tag.unit = Unit::Pixels;
                        match code {
                            PLUS | MINUS | PERIOD | DIGIT_ZERO..=DIGIT_NINE => {
                                kind = ValueKind::Number;
                                attribute.start = tag.length - 1;
                                attribute.length += 1;
                            }
                            HASH => {
                                kind = ValueKind::Color;
                                attribute.start = tag.length - 1;
                                attribute.length += 1;
                            }
                            QUOTE => {
                                kind = ValueKind::String;
                                attribute.start = tag.length;
                            }
                            _ => {
                                kind = ValueKind::String;
                                attribute.start = tag.length - 1;
                                attribute.value = attribute.value.push(code);
                                attribute.length += 1;
                            }
                        }

                        attribute.kind = kind;
                    }
                    ValueKind::Number => {
                        if matches!(code, PIXELS | EM | PERCENT | SPACE) {
                            phase = Phase::Between;
                            kind = ValueKind::None;
                            tag.unit = match code {
                                EM => Unit::FontUnits,
                                PERCENT => Unit::Percentage,
                                _ => Unit::Pixels,
                            };
                            attribute.unit = tag.unit;
                            current = tag.next_attribute(current)?;
                        } else {
                            attribute.length += 1;
                        }
                    }
                    ValueKind::Color => {
                        if code == SPACE {
                            phase = Phase::Between;
                            kind = ValueKind::None;
                            tag.unit = Unit::Pixels;
                            current = tag.next_attribute(current)?;
                        } else {
                            attribute.length += 1;
                        }
                    }
                    ValueKind::String => {
                        if code == QUOTE {
                            phase = Phase::Between;
                            kind = ValueKind::None;
                            tag.unit = Unit::Pixels;
                            current = tag.next_attribute(current)?;
                        } else {
                            attribute.value = attribute.value.push(code);
                            attribute.length += 1;
                        }
                    }
                }
            }

            if code == EQUALS {
                phase = Phase::Value;
            }

            if phase == Phase::Name && code == SPACE {
                if spaced {
                    return None;
                }

                spaced = true;
                phase = Phase::Between;
                kind = ValueKind::None;
                tag.unit = Unit::Pixels;
                current = tag.next_attribute(current)?;
            }

            if phase == Phase::Name {
                let attribute = &mut tag.attributes[current];

                attribute.name = attribute.name.push(code);
            }

            if phase == Phase::Between && code == SPACE {
                phase = Phase::Name;
            }
        }

        closed.then_some(tag)
    }

    fn next_attribute(&mut self, current: usize) -> Option<usize> {
        let next = current + 1;

        *self.attributes.get_mut(next)? = Attribute::default();

        Some(next)
    }

    pub(crate) fn name(&self) -> i32 {
        self.attributes[0].name.0
    }

    pub(crate) fn number(&self, attribute: usize) -> f32 {
        let attribute = &self.attributes[attribute];

        TagNumber::parse(&self.text, attribute.start, attribute.length)
    }

    pub(crate) fn parameters(&self, attribute: usize) -> Vec<f32> {
        let attribute = &self.attributes[attribute];

        TagNumber::parameters(&self.text, attribute.start, attribute.length)
    }

    pub(crate) fn value_text(&self, attribute: usize) -> String {
        let attribute = &self.attributes[attribute];
        let end = (attribute.start + attribute.length).min(self.text.len());

        String::from_utf16_lossy(&self.text[attribute.start.min(end)..end])
    }

    pub(crate) fn color(&self, attribute: usize) -> Color {
        let attribute = &self.attributes[attribute];

        Color::from_value(&self.text, attribute.start, attribute.length)
    }

    pub(crate) fn listed_attributes(&self) -> impl Iterator<Item = (usize, &Attribute)> {
        self.attributes
            .iter()
            .enumerate()
            .take_while(|(_, attribute)| attribute.name.0 != 0)
    }
}
