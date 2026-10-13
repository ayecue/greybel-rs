use text_mesh_core::{
    Color, FontWeight, LayoutChange, Length, LineHeight, Run, Style, Text, TextAssetTable,
};
use text_mesh_font::{FontSet, GlyphAdjustment, GlyphMetrics};

use crate::layout::ElementKind;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Token {
    Element(usize),
    Position(Length),
    Space(f32),
    Layout(LayoutChange),
    CharacterSpacingEnd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SourceElement {
    pub kind: ElementKind,
    pub glyph: GlyphMetrics,
    pub element_scale: f32,
    pub glyph_scale: f32,
    pub style: Style,
    pub font: usize,
    pub style_font: Option<usize>,
}

impl SourceElement {
    pub(crate) const fn unicode(&self) -> u32 {
        match self.kind {
            ElementKind::Glyph { character, .. } => character as u32,
            ElementKind::Sprite { index, .. } => 0xE000 + index,
        }
    }

    pub(crate) const fn is_sprite(&self) -> bool {
        matches!(self.kind, ElementKind::Sprite { .. })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Source {
    pub tokens: Vec<Token>,
    pub elements: Vec<SourceElement>,
    fonts: Vec<Option<usize>>,
}

impl Source {
    pub(crate) fn new(text: &Text, fonts: &FontSet, assets: &TextAssetTable, size: f32) -> Self {
        let mut builder = SourceBuilder {
            fonts,
            assets,
            tokens: Vec::new(),
            elements: Vec::new(),
            style: text.runs.iter().find_map(Self::style).unwrap_or(Style {
                color: Color::WHITE,
                bold: false,
                weight: FontWeight::REGULAR,
                italic: false,
                underline: None,
                strikethrough: None,
                highlight: None,
                size,
                subscript: false,
                superscript: false,
                small_caps: false,
                character_spacing: 0.0,
                mono_spacing: 0.0,
                vertical_offset: 0.0,
                rotation: 0.0,
                scale: 1.0,
                font: None,
                material: None,
            }),
            spacing_end: None,
        };

        for run in &text.runs {
            builder.push(run);
        }

        Self {
            tokens: builder.tokens,
            elements: builder.elements,
            fonts: assets.fonts.iter().map(|name| fonts.find(name)).collect(),
        }
    }

    pub(crate) fn kerning(&self, fonts: &FontSet, index: usize) -> GlyphAdjustment {
        let mut adjustment = GlyphAdjustment::default();
        let ElementKind::Glyph { character, font } = self.elements[index].kind else {
            return adjustment;
        };
        let asset = fonts.font(font);
        let neighbour = |index: usize| match self.elements.get(index)?.kind {
            ElementKind::Glyph {
                character,
                font: other,
            } if other == font => Some(character),
            _ => None,
        };

        if let Some(next) = neighbour(index + 1)
            && let Some(pair) = asset.kerning(character, next)
        {
            adjustment = pair.first;
        }

        if let Some(previous) = index.checked_sub(1).and_then(neighbour)
            && let Some(pair) = asset.kerning(previous, character)
        {
            adjustment = GlyphAdjustment {
                x_placement: adjustment.x_placement + pair.second.x_placement,
                y_placement: adjustment.y_placement + pair.second.y_placement,
                x_advance: adjustment.x_advance + pair.second.x_advance,
                y_advance: adjustment.y_advance + pair.second.y_advance,
            };
        }

        adjustment
    }

    pub(crate) fn tag_font(&self, change: &LayoutChange) -> Option<usize> {
        match *change {
            LayoutChange::LineHeight(LineHeight::Percentage {
                font: Some(font), ..
            }) => self.fonts.get(usize::from(font)).copied().flatten(),
            _ => None,
        }
    }

    fn style(run: &Run) -> Option<Style> {
        match run {
            Run::Text { style, .. } | Run::Sprite { style, .. } => Some(*style),
            _ => None,
        }
    }
}

struct SourceBuilder<'a> {
    fonts: &'a FontSet,
    assets: &'a TextAssetTable,
    tokens: Vec<Token>,
    elements: Vec<SourceElement>,
    style: Style,
    spacing_end: Option<usize>,
}

impl SourceBuilder<'_> {
    fn push(&mut self, run: &Run) {
        match run {
            Run::Text { text, style } => {
                for character in text.chars() {
                    self.glyph(character, *style);
                }
            }
            Run::Sprite {
                index,
                color,
                style,
            } => self.sprite(*index, *color, *style),
            Run::Position { x } => self.tokens.push(Token::Position(*x)),
            Run::Space { width } => self.tokens.push(Token::Space(*width)),
            Run::Layout(change) => self.tokens.push(Token::Layout(*change)),
            Run::LineBreak => self.glyph('\n', self.style),
        }
    }

    fn glyph(&mut self, character: char, style: Style) {
        let style_font = self.style_font(&style);
        let base = style_font.unwrap_or(self.fonts.primary);
        let (character, font, glyph) = self.fonts.element(base, character);

        self.element(SourceElement {
            kind: ElementKind::Glyph { character, font },
            glyph,
            element_scale: 1.0,
            glyph_scale: 1.0,
            style,
            font,
            style_font,
        });
    }

    fn sprite(&mut self, index: u32, color: Color, style: Style) {
        let Some((character, glyph)) = self.fonts.sprites.sprite(index as usize) else {
            return;
        };
        let style_font = self.style_font(&style);

        self.element(SourceElement {
            kind: ElementKind::Sprite { index, color },
            glyph: glyph.metrics,
            element_scale: character.scale,
            glyph_scale: glyph.scale,
            style,
            font: style_font.unwrap_or(self.fonts.primary),
            style_font,
        });
    }

    fn element(&mut self, element: SourceElement) {
        let spacing = element.style.character_spacing;

        if spacing == 0.0
            && let Some(position) = self.spacing_end.take()
        {
            self.tokens.insert(position, Token::CharacterSpacingEnd);
        }

        self.style = element.style;
        self.tokens.push(Token::Element(self.elements.len()));
        self.elements.push(element);
        if spacing != 0.0 {
            self.spacing_end = Some(self.tokens.len());
        }
    }

    fn style_font(&self, style: &Style) -> Option<usize> {
        let name = self.assets.fonts.get(usize::from(style.font?))?;

        self.fonts.find(name)
    }
}

#[cfg(test)]
#[path = "tests/source.rs"]
mod tests;
