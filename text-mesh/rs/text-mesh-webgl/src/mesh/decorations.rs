use text_mesh_core::Color;
use text_mesh_font::{FontAsset, FontSet, GlyphMetrics, GlyphRect};

use crate::mesh::Vertex;
use crate::mesh::character::Character;
use crate::mesh::point::Point;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineEnds {
    pub last: usize,
    pub last_visible: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Underline {
    active: bool,
    start: Point,
    end: Point,
    start_scale: f32,
    end_scale: f32,
    max_scale: f32,
    x_scale_max: f32,
    baseline: f32,
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Strikethrough {
    active: bool,
    start: Point,
    point_size: f32,
    scale: f32,
    baseline: f32,
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Highlight {
    active: bool,
    start: Point,
    end: Point,
    color: Color,
}

/// Underline glyph TMP draws underlines, strikethroughs and highlights with: the primary font's own `_`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct UnderlineGlyph {
    metrics: GlyphMetrics,
    rect: GlyphRect,
    atlas_width: f32,
    atlas_height: f32,
    thickness: f32,
    offset: f32,
    padding: f32,
}

impl UnderlineGlyph {
    fn of(font: &FontAsset) -> Option<Self> {
        let metrics = font.glyph('_')?;
        let rect = font.rect('_')?;
        let (atlas_width, atlas_height) = font.atlas.map_or((1.0, 1.0), |atlas| {
            (f32::from(atlas.width), f32::from(atlas.height))
        });

        Some(Self {
            metrics,
            rect,
            atlas_width,
            atlas_height,
            thickness: font.face.underline_thickness,
            offset: font.face.underline_offset,
            padding: font.padding(),
        })
    }
}

/// Tracks underline, strikethrough, and highlight geometry during TMP's second pass in mesh space.
pub(crate) struct DecorationManager<'a> {
    fonts: &'a FontSet,
    font_color: Color,
    characters: &'a [Character],
    lines: &'a [LineEnds],
    offset: [f32; 2],
    glyph: Option<UnderlineGlyph>,
    underline: Underline,
    strikethrough: Strikethrough,
    highlight: Highlight,
    vertices: Vec<Vertex>,
}

impl<'a> DecorationManager<'a> {
    const LARGE_NEGATIVE: f32 = -32767.0;
    const LARGE_POSITIVE: f32 = 32767.0;

    pub(crate) fn new(
        fonts: &'a FontSet,
        font_color: Color,
        characters: &'a [Character],
        lines: &'a [LineEnds],
        offset: [f32; 2],
    ) -> Self {
        Self {
            fonts,
            font_color,
            characters,
            lines,
            offset,
            glyph: UnderlineGlyph::of(fonts.font(fonts.primary)),
            underline: Underline {
                active: false,
                start: Point::default(),
                end: Point::default(),
                start_scale: 0.0,
                end_scale: 0.0,
                max_scale: 0.0,
                x_scale_max: 0.0,
                baseline: Self::LARGE_POSITIVE,
                color: Color::WHITE,
            },
            strikethrough: Strikethrough {
                active: false,
                start: Point::default(),
                point_size: 0.0,
                scale: 0.0,
                baseline: 0.0,
                color: Color::WHITE,
            },
            highlight: Highlight {
                active: false,
                start: Point::default(),
                end: Point::default(),
                color: Color::WHITE,
            },
            vertices: Vec::new(),
        }
    }

    pub(crate) fn into_vertices(self) -> Vec<Vertex> {
        self.vertices
    }

    /// Processes character `index` after quad generation; `x_scale` is the latest visible glyph's signed scale.
    pub(crate) fn character(&mut self, index: usize, x_scale: f32) {
        self.underline(index, x_scale);
        self.strikethrough(index, x_scale);
        self.highlight(index);
    }

    fn underline(&mut self, index: usize, x_scale: f32) {
        let character = self.characters[index];
        let line = self.lines[character.line];
        let count = self.characters.len();
        let Some(color) = character.style.underline else {
            if self.underline.active {
                let previous = self.characters[index - 1];

                self.underline.end = Point::new(previous.top_right.x, self.underline.baseline);
                self.underline.end_scale = previous.scale;
                self.end_underline();
            }

            return;
        };

        if !character.is_white_space() && character.code != 0x200B {
            let underline = &mut self.underline;

            underline.max_scale = underline.max_scale.max(character.scale);
            underline.x_scale_max = underline.x_scale_max.max(x_scale.abs());

            let offset = self.glyph.map_or(0.0, |glyph| glyph.offset);

            underline.baseline = underline
                .baseline
                .min(character.baseline + offset * underline.max_scale);
        }

        if !self.underline.active
            && index <= line.last_visible
            && !character.is_line_break()
            && !(index == line.last_visible && character.is_separator())
        {
            let underline = &mut self.underline;

            underline.active = true;
            underline.start_scale = character.scale;
            if underline.max_scale == 0.0 {
                underline.max_scale = underline.start_scale;
                underline.x_scale_max = x_scale;
            }

            underline.start = Point::new(character.bottom_left.x, underline.baseline);
            underline.color = color;
        }

        if !self.underline.active {
            return;
        }

        if count == 1 {
            self.underline.end = Point::new(character.top_right.x, self.underline.baseline);
            self.underline.end_scale = character.scale;
            self.end_underline();
        } else if index == line.last || index >= line.last_visible {
            let end = if character.is_white_space() || character.code == 0x200B {
                self.characters[line.last_visible]
            } else {
                character
            };

            self.underline.end = Point::new(end.top_right.x, self.underline.baseline);
            self.underline.end_scale = end.scale;
            self.end_underline();
        } else if self
            .characters
            .get(index + 1)
            .and_then(|next| next.style.underline)
            .is_some_and(|next| next != self.underline.color)
        {
            self.underline.end = Point::new(character.top_right.x, self.underline.baseline);
            self.underline.end_scale = character.scale;
            self.end_underline();
        }
    }

    fn end_underline(&mut self) {
        let underline = self.underline;

        self.underline_mesh(
            underline.start,
            underline.end,
            [
                underline.start_scale,
                underline.end_scale,
                underline.max_scale,
            ],
            underline.x_scale_max,
            underline.color,
        );
        self.underline.active = false;
        self.underline.max_scale = 0.0;
        self.underline.x_scale_max = 0.0;
        self.underline.baseline = Self::LARGE_POSITIVE;
    }

    fn strikethrough(&mut self, index: usize, x_scale: f32) {
        let character = self.characters[index];
        let line = self.lines[character.line];
        let count = self.characters.len();
        let offset = self.fonts.font(character.font).face.strikethrough_offset;
        let Some(color) = character.style.strikethrough else {
            if self.strikethrough.active {
                let previous = self.characters[index - 1];

                self.end_strikethrough(previous, offset, x_scale);
            }

            return;
        };

        if !self.strikethrough.active
            && index <= line.last_visible
            && !character.is_line_break()
            && !(index == line.last_visible && character.is_separator())
        {
            let strikethrough = &mut self.strikethrough;

            strikethrough.active = true;
            strikethrough.point_size = character.point_size;
            strikethrough.scale = character.scale;
            strikethrough.start = Point::new(
                character.bottom_left.x,
                character.baseline + offset * strikethrough.scale,
            );
            strikethrough.color = color;
            strikethrough.baseline = character.baseline;
        }

        if !self.strikethrough.active {
            return;
        }

        let next = self.characters.get(index + 1);

        if count == 1 {
            self.end_strikethrough(character, offset, x_scale);
        } else if index == line.last {
            let end = if character.is_white_space() || character.code == 0x200B {
                self.characters[line.last_visible]
            } else {
                character
            };

            self.end_strikethrough(end, offset, x_scale);
        } else if next.is_some_and(|next| {
            next.point_size != self.strikethrough.point_size
                || !Self::approximately(next.baseline, self.strikethrough.baseline)
        }) {
            let end = if index > line.last_visible {
                self.characters[line.last_visible]
            } else {
                character
            };

            self.end_strikethrough(end, offset, x_scale);
        } else if next.is_some_and(|next| next.font != character.font) {
            self.end_strikethrough(character, offset, x_scale);
        }
    }

    fn end_strikethrough(&mut self, end: Character, offset: f32, x_scale: f32) {
        let strikethrough = self.strikethrough;
        let scale = strikethrough.scale;
        let end = Point::new(end.top_right.x, end.baseline + offset * scale);

        self.strikethrough.active = false;
        self.underline_mesh(
            strikethrough.start,
            end,
            [scale, scale, scale],
            x_scale,
            strikethrough.color,
        );
    }

    fn highlight(&mut self, index: usize) {
        let character = self.characters[index];
        let line = self.lines[character.line];
        let count = self.characters.len();
        let Some(color) = character.style.highlight else {
            if self.highlight.active {
                self.end_highlight();
            }

            return;
        };

        if !self.highlight.active
            && index <= line.last_visible
            && !character.is_line_break()
            && !(index == line.last_visible && character.is_separator())
        {
            self.highlight = Highlight {
                active: true,
                start: Point::new(Self::LARGE_POSITIVE, Self::LARGE_POSITIVE),
                end: Point::new(Self::LARGE_NEGATIVE, Self::LARGE_NEGATIVE),
                color,
            };
        }

        if !self.highlight.active {
            return;
        }

        let highlight = &mut self.highlight;

        if highlight.color == color {
            highlight.start.x = highlight.start.x.min(character.bottom_left.x);
            highlight.start.y = highlight.start.y.min(character.descender);
            highlight.end.x = highlight.end.x.max(character.top_right.x);
            highlight.end.y = highlight.end.y.max(character.ascender);
        } else {
            highlight.end.x = (highlight.end.x + character.bottom_left.x) / 2.0;
            highlight.start.y = highlight.start.y.min(character.descender);
            highlight.end.y = highlight.end.y.max(character.ascender);

            let previous = *highlight;

            highlight.start = Point::new(previous.end.x, character.descender);
            highlight.end = Point::new(character.top_right.x, character.ascender);
            highlight.color = color;
            self.highlight_mesh(previous.start, previous.end, previous.color);
        }

        if count == 1 || index == line.last || index >= line.last_visible {
            self.end_highlight();
        }
    }

    fn end_highlight(&mut self) {
        let highlight = self.highlight;

        self.highlight.active = false;
        self.highlight_mesh(highlight.start, highlight.end, highlight.color);
    }

    fn underline_mesh(
        &mut self,
        mut start: Point,
        mut end: Point,
        [start_scale, end_scale, max_scale]: [f32; 3],
        sdf_scale: f32,
        color: Color,
    ) {
        let Some(glyph) = self.glyph else {
            return;
        };

        start.y = start.y.min(end.y);
        end.y = start.y.min(end.y);

        let padding = glyph.padding;
        let mut segment_width = glyph.metrics.width / 2.0 * max_scale;

        if end.x - start.x < glyph.metrics.width * max_scale {
            segment_width = (end.x - start.x) / 2.0;
        }

        let start_padding = padding * start_scale / max_scale;
        let end_padding = padding * end_scale / max_scale;
        let thickness = glyph.thickness;
        let below = 0.0 - (thickness + padding) * max_scale;
        let above = padding * max_scale;

        let bottom_left = start + Point::new(0.0, below);
        let top_left = start + Point::new(0.0, above);
        let top_middle_left = top_left + Point::new(segment_width, 0.0);
        let bottom_middle_left = bottom_left + Point::new(segment_width, 0.0);
        let top_middle_right = end + Point::new(-segment_width, above);
        let bottom_middle_right =
            end + Point::new(-segment_width, -(thickness + padding) * max_scale);
        let top_right = end + Point::new(0.0, above);
        let bottom_right = end + Point::new(0.0, -(thickness + padding) * max_scale);
        let positions = [
            bottom_left,
            top_left,
            top_middle_left,
            bottom_middle_left,
            bottom_middle_left,
            top_middle_left,
            top_middle_right,
            bottom_middle_right,
            bottom_middle_right,
            top_middle_right,
            top_right,
            bottom_right,
        ];

        let rect = glyph.rect;
        let (x, y, width) = (f32::from(rect.x), f32::from(rect.y), f32::from(rect.width));
        let (atlas_width, atlas_height) = (glyph.atlas_width, glyph.atlas_height);
        let bottom = (y - padding) / atlas_height;
        let top = (f32::from(rect.y + rect.height) + padding) / atlas_height;
        let left = (x - start_padding) / atlas_width;
        let middle_left = (x - start_padding + width / 2.0) / atlas_width;
        let middle_right = (x + end_padding + width / 2.0) / atlas_width;
        let right = (x + end_padding + width) / atlas_width;
        let narrow_left = middle_left - middle_left * 0.001;
        let narrow_right = middle_left + middle_left * 0.001;
        let uvs = [
            [left, bottom],
            [left, top],
            [middle_left, top],
            [middle_left, bottom],
            [narrow_left, bottom],
            [narrow_left, top],
            [narrow_right, top],
            [narrow_right, bottom],
            [middle_right, bottom],
            [middle_right, top],
            [right, top],
            [right, bottom],
        ];

        let color = self.vertex_color(color);
        let scale = sdf_scale.abs();
        let offset = self.offset;

        self.vertices
            .extend(positions.iter().zip(uvs).map(|(position, uv)| Vertex {
                position: position.to_layout(offset),
                uv,
                scale,
                color,
            }));
    }

    fn highlight_mesh(&mut self, start: Point, end: Point, color: Color) {
        let Some(glyph) = self.glyph else {
            return;
        };
        let rect = glyph.rect;
        let uv = [
            (f32::from(rect.x) + f32::from(rect.width / 2)) / glyph.atlas_width,
            (f32::from(rect.y) + f32::from(rect.height) / 2.0) / glyph.atlas_height,
        ];
        let positions = [
            start,
            Point::new(start.x, end.y),
            end,
            Point::new(end.x, start.y),
        ];
        let color = self.vertex_color(color);
        let offset = self.offset;

        self.vertices
            .extend(positions.iter().map(|position| Vertex {
                position: position.to_layout(offset),
                uv,
                scale: 1.0,
                color,
            }));
    }

    fn vertex_color(&self, color: Color) -> Color {
        Color {
            a: color.a.min(self.font_color.a),
            ..color
        }
    }

    fn approximately(a: f32, b: f32) -> bool {
        (b - 0.0001) < a && a < (b + 0.0001)
    }
}

#[cfg(test)]
#[path = "tests/decorations.rs"]
mod tests;
