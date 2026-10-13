use std::borrow::Cow;

use crate::hash::{TagHash, ValueHash};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStyle {
    pub name: Cow<'static, str>,
    pub open_markup: Cow<'static, str>,
    pub close_markup: Cow<'static, str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAssetTable {
    pub fonts: Cow<'static, [Cow<'static, str>]>,
    pub materials: Cow<'static, [Cow<'static, str>]>,
    pub sprites: Cow<'static, [Cow<'static, str>]>,
    pub styles: Cow<'static, [TextStyle]>,
}

impl TextAssetTable {
    pub const EMPTY: Self = Self {
        fonts: Cow::Borrowed(&[]),
        materials: Cow::Borrowed(&[]),
        sprites: Cow::Borrowed(&[]),
        styles: Cow::Borrowed(&[]),
    };

    pub(crate) fn style(&self, hash: i32) -> Option<&TextStyle> {
        self.styles
            .iter()
            .find(|style| TagHash::of(&style.name) == hash)
    }

    pub(crate) fn font(&self, name: &str) -> Option<u16> {
        Self::position(&self.fonts, name)
    }

    pub(crate) fn material(&self, name: &str) -> Option<u16> {
        Self::position(&self.materials, name)
    }

    pub(crate) fn sprite(&self, value: i32) -> Option<usize> {
        self.sprites
            .iter()
            .position(|name| ValueHash::of(name) == value)
    }

    pub(crate) fn sprite_count(&self) -> u32 {
        u32::try_from(self.sprites.len()).unwrap_or(u32::MAX)
    }

    fn position(names: &[Cow<'static, str>], name: &str) -> Option<u16> {
        names
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(name))
            .and_then(|index| u16::try_from(index).ok())
    }
}

impl Default for TextAssetTable {
    fn default() -> Self {
        Self::EMPTY
    }
}
