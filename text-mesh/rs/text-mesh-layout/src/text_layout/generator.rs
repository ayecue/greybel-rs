use text_mesh_font::{FontSet, GlyphAdjustment};

use crate::settings::TextSettings;
use crate::text_layout::cursor::{Cursor, WrapState};
use crate::text_layout::line_break::LineBreak;
use crate::text_layout::metrics::ElementMetrics;
use crate::text_layout::records::{CharacterInfo, EffectTransform, Extents, LineInfo, Point};
use crate::text_layout::source::{Source, Token};
use crate::text_layout::tags::TagContext;
use crate::text_layout::{mono_sum, unicode};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GeneratedText {
    pub characters: Vec<CharacterInfo>,
    pub lines: Vec<LineInfo>,
    pub line_number: usize,
    pub max_text_ascender: f32,
    pub max_visible_descender: f32,
    pub max_cap_height: f32,
    pub mesh: Extents,
}

#[derive(Debug, Clone, Copy)]
struct Advance {
    adjustment: GlyphAdjustment,
    character_spacing: f32,
    bold_spacing: f32,
}

pub(crate) struct Generator<'a> {
    fonts: &'a FontSet,
    settings: &'a TextSettings,
    source: &'a Source,
    margin_width: f32,
    base_scale: f32,
    em_scale: f32,
    line_gap: f32,
    cursor: Cursor,
    characters: Vec<CharacterInfo>,
    lines: Vec<LineInfo>,
    index: isize,
    character_count: usize,
    last_character_of_line: usize,
    width_of_text_area: f32,
    is_start_of_new_line: bool,
    is_first_word_of_line: bool,
    is_soft_hyphen_ignored: bool,
    last_soft_line_break: isize,
    max_visible_descender: f32,
    mesh: Extents,
    word_wrap: WrapState,
    soft_line_break: WrapState,
}

impl<'a> Generator<'a> {
    pub(crate) fn new(
        fonts: &'a FontSet,
        settings: &'a TextSettings,
        source: &'a Source,
        margin_width: f32,
    ) -> Self {
        let primary = fonts.font(fonts.primary);
        let face = &primary.face;
        let cursor = Cursor::new(settings.alignment, fonts.primary);
        let line = LineInfo::new(settings.alignment);
        let initial = WrapState::new(-1, -1, &cursor, line, Extents::EMPTY);

        Self {
            fonts,
            settings,
            source,
            margin_width,
            base_scale: settings.font_size / face.point_size * face.scale,
            em_scale: settings.font_size * 0.01,
            line_gap: face.line_height - (face.ascent_line - face.descent_line),
            cursor,
            characters: vec![CharacterInfo::default(); source.elements.len()],
            lines: vec![line],
            index: 0,
            character_count: 0,
            last_character_of_line: 0,
            width_of_text_area: margin_width + 0.0001,
            is_start_of_new_line: true,
            is_first_word_of_line: true,
            is_soft_hyphen_ignored: false,
            last_soft_line_break: 0,
            max_visible_descender: 0.0,
            mesh: Extents::EMPTY,
            word_wrap: initial.clone(),
            soft_line_break: initial,
        }
    }

    pub(crate) fn generate(mut self) -> GeneratedText {
        let tokens = &self.source.tokens;

        while self.index < tokens.len() as isize {
            match tokens[self.index as usize] {
                Token::Element(element) => self.element(element),
                Token::Position(x) => self.cursor.tags.position(x, self.margin_width),
                Token::Space(width) => self.cursor.tags.add_space(width),
                Token::Layout(change) => {
                    let context = TagContext {
                        fonts: self.fonts,
                        margin_width: self.margin_width,
                        current_font: self.cursor.current_font,
                        tag_font: self.source.tag_font(&change),
                    };

                    self.cursor.tags.apply(&change, &context);
                }
                Token::CharacterSpacingEnd => self.end_character_spacing(),
            }

            self.index += 1;
        }

        self.characters.truncate(self.character_count);

        GeneratedText {
            characters: self.characters,
            lines: self.lines,
            line_number: self.cursor.line_number,
            max_text_ascender: self.cursor.max_text_ascender,
            max_visible_descender: self.max_visible_descender,
            max_cap_height: self.cursor.max_cap_height,
            mesh: self.mesh,
        }
    }

    /// Computes TMP `lineOffsetDelta` for the next line using Mono-compatible double-precision evaluation before float storage.
    fn line_offset_delta(&self, ascender: f32, spacing: f64) -> f32 {
        (0.0 - f64::from(self.cursor.max_line_descender)
            + f64::from(ascender)
            + f64::from(self.line_gap) * f64::from(self.base_scale)
            + spacing) as f32
    }

    fn end_character_spacing(&mut self) {
        let tags = &mut self.cursor.tags;

        if self.character_count > 0 {
            tags.x_advance -= tags.character_spacing;
            self.characters[self.character_count - 1].x_advance = tags.x_advance;
        }

        tags.character_spacing = 0.0;
    }

    fn element(&mut self, element_index: usize) {
        let element = self.source.elements[element_index];
        let style = element.style;
        let total = self.source.elements.len();
        let count = self.character_count;
        let metrics = ElementMetrics::new(self.fonts, &element, self.cursor.font_for(&element));
        let font = self.fonts.font(metrics.font);
        let code = metrics.character;
        let is_sprite = element.is_sprite();

        if !is_sprite {
            self.cursor.current_font = metrics.font;
            self.cursor.current_base_font = element.style_font.unwrap_or(self.fonts.primary);
        }

        if style.italic {
            if self.cursor.italic_angle.is_none() {
                self.cursor.italic_angle = Some(f32::from(font.style.italic_style));
            }
        } else {
            self.cursor.italic_angle = None;
        }

        self.cursor.tags.character_spacing = style.character_spacing;
        self.cursor.tags.mono_spacing = style.mono_spacing;

        let mut scale = metrics.scale;
        let unmodified_scale = scale;

        if code == unicode::SOFT_HYPHEN || code == unicode::END_OF_TEXT {
            scale = 0.0;
        }

        let mut padding = metrics.padding;
        let glyph = element.glyph;
        let is_white_space = metrics.is_white_space();

        {
            let character = &mut self.characters[count];

            character.character = code;
            character.point_size = style.size;
            character.scale = unmodified_scale;
        }

        let advance = self.advance(count);
        let mono_spacing = self.cursor.tags.mono_spacing;
        let mut mono_advance = 0.0;

        if mono_spacing != 0.0 {
            mono_advance = mono_spacing / 2.0 - (glyph.width / 2.0 + glyph.bearing_x) * scale;
            self.cursor.tags.x_advance += mono_advance;
        }

        let mut style_padding = 0.0;
        let mut bold_spacing = 0.0;

        if !is_sprite && style.bold {
            if let Some(material) = metrics.material {
                let gradient_scale = material.gradient_scale;

                style_padding =
                    font.style.bold_style / 4.0 * gradient_scale * material.scale_ratio_a();
                if style_padding + padding > gradient_scale {
                    padding = gradient_scale - style_padding;
                }
            }

            bold_spacing = font.style.bold_spacing;
        } else if let Some(material) = metrics.material {
            let gradient_scale = material.gradient_scale;

            style_padding =
                font.style.normal_style / 4.0 * gradient_scale * material.scale_ratio_a();
            if style_padding + padding > gradient_scale {
                padding = gradient_scale - style_padding;
            }
        }

        let advance = Advance {
            bold_spacing,
            ..advance
        };

        let x_advance = self.cursor.tags.x_advance;
        let line_offset = self.cursor.line_offset;
        let baseline_offset = metrics.baseline_offset;
        let mut top_left = Point::new(
            x_advance
                + ((glyph.bearing_x - padding - style_padding + advance.adjustment.x_placement)
                    * scale),
            metrics.element_baseline
                + (glyph.bearing_y + padding + advance.adjustment.y_placement) * scale
                - line_offset
                + baseline_offset,
        );
        let mut bottom_left = Point::new(
            top_left.x,
            top_left.y - ((glyph.height + padding * 2.0) * scale),
        );
        let mut top_right = Point::new(
            bottom_left.x + ((glyph.width + padding * 2.0 + style_padding * 2.0) * scale),
            top_left.y,
        );
        let mut bottom_right = Point::new(top_right.x, bottom_left.y);

        if !is_sprite
            && style.italic
            && let Some(angle) = self.cursor.italic_angle
        {
            let shear = angle * 0.01;
            let top_shear = Point::new(
                shear * ((glyph.bearing_y + padding + style_padding) * scale),
                0.0,
            );
            let bottom_shear = Point::new(
                shear * ((glyph.bearing_y - glyph.height - padding - style_padding) * scale),
                0.0,
            );
            let adjustment = Point::new((top_shear.x - bottom_shear.x) / 2.0, 0.0);

            top_left = top_left + top_shear - adjustment;
            bottom_left = bottom_left + bottom_shear - adjustment;
            top_right = top_right + top_shear - adjustment;
            bottom_right = bottom_right + bottom_shear - adjustment;
        }

        let transform = EffectTransform::new(style.rotation, style.scale);

        if let Some(transform) = transform {
            let offset = (top_right + bottom_left) / 2.0;

            top_left = transform.apply(top_left - offset) + offset;
            bottom_left = transform.apply(bottom_left - offset) + offset;
            top_right = transform.apply(top_right - offset) + offset;
            bottom_right = transform.apply(bottom_right - offset) + offset;
        }

        {
            let character = &mut self.characters[count];

            character.bottom_left = bottom_left;
            character.top_left = top_left;
            character.top_right = top_right;
            character.bottom_right = bottom_right;
            character.origin = x_advance;
            character.baseline = metrics.element_baseline - line_offset + baseline_offset;
            character.font = metrics.font;
            character.padding = padding;
            character.style_padding = style_padding;
        }

        let (element_ascender, element_descender) = if is_sprite {
            (
                metrics.ascent_line * scale + baseline_offset,
                metrics.descent_line * scale + baseline_offset,
            )
        } else {
            (
                metrics.ascent_line * scale / metrics.small_caps + baseline_offset,
                metrics.descent_line * scale / metrics.small_caps + baseline_offset,
            )
        };
        let mut adjusted_ascender = element_ascender;
        let mut adjusted_descender = element_descender;
        let is_first_character_of_line = count == self.cursor.first_character_of_line;
        let counts_for_line = is_first_character_of_line || !is_white_space;

        if counts_for_line {
            if baseline_offset != 0.0 {
                adjusted_ascender = ((element_ascender - baseline_offset)
                    / metrics.font_scale_multiplier)
                    .max(adjusted_ascender);
                adjusted_descender = ((element_descender - baseline_offset)
                    / metrics.font_scale_multiplier)
                    .min(adjusted_descender);
            }

            self.cursor.max_line_ascender = adjusted_ascender.max(self.cursor.max_line_ascender);
            self.cursor.max_line_descender = adjusted_descender.min(self.cursor.max_line_descender);
        }

        {
            let cursor = &mut self.cursor;
            let character = &mut self.characters[count];

            if counts_for_line {
                character.adjusted_ascender = adjusted_ascender;
                character.adjusted_descender = adjusted_descender;
                character.ascender = element_ascender - line_offset;
                character.descender = element_descender - line_offset;
            } else {
                character.adjusted_ascender = cursor.max_line_ascender;
                character.adjusted_descender = cursor.max_line_descender;
                character.ascender = cursor.max_line_ascender - line_offset;
                character.descender = cursor.max_line_descender - line_offset;
            }

            cursor.element_descender = character.descender;
            if cursor.line_number == 0 && counts_for_line {
                cursor.max_text_ascender = cursor.max_line_ascender;
                cursor.max_cap_height = cursor
                    .max_cap_height
                    .max(font.cap_line() * scale / metrics.small_caps);
            }
        }

        let mut visible = false;

        if code == unicode::TAB
            || (!is_white_space
                && code != unicode::ZERO_WIDTH_SPACE
                && code != unicode::SOFT_HYPHEN
                && code != unicode::END_OF_TEXT)
            || (code == unicode::SOFT_HYPHEN && !self.is_soft_hyphen_ignored)
            || is_sprite
        {
            visible = true;

            let tags = &self.cursor.tags;
            let margin_left = tags.margin_left;
            let margin_right = tags.margin_right;

            self.width_of_text_area = tags.width_of_text_area(self.margin_width);

            let width_scale = if code == unicode::SOFT_HYPHEN {
                unmodified_scale
            } else {
                scale
            };
            let text_width = x_advance.abs() + glyph.advance * width_scale;
            let limit = if tags.is_justified_or_flush() {
                1.05
            } else {
                1.0
            };

            if text_width > self.width_of_text_area * limit
                && self.settings.word_wrapping
                && count != self.cursor.first_character_of_line
            {
                self.characters[count].visible = true;
                self.wrap(scale, advance);

                return;
            }

            if code == unicode::TAB {
                visible = false;
                self.cursor.last_visible_character_of_line = count;
                self.lines[self.cursor.line_number].whitespace_count += 1;
            } else if code == unicode::SOFT_HYPHEN {
                visible = false;
            } else {
                if self.is_start_of_new_line {
                    self.is_start_of_new_line = false;
                    self.cursor.first_visible_character_of_line = count;
                }

                self.cursor.line_visible_character_count += 1;
                self.cursor.last_visible_character_of_line = count;

                let line = &mut self.lines[self.cursor.line_number];

                line.margin_left = margin_left;
                line.margin_right = margin_right;
            }
        } else {
            let line = &mut self.lines[self.cursor.line_number];

            if (matches!(
                code,
                unicode::LF
                    | unicode::VT
                    | unicode::NO_BREAK_SPACE
                    | unicode::FIGURE_SPACE
                    | unicode::LINE_SEPARATOR
                    | unicode::PARAGRAPH_SEPARATOR
            ) || LineBreak::is_separator(code))
                && code != unicode::SOFT_HYPHEN
                && code != unicode::ZERO_WIDTH_SPACE
                && code != unicode::WORD_JOINER
            {
                line.whitespace_count += 1;
            }

            if code == unicode::NO_BREAK_SPACE {
                line.control_character_count += 1;
            }
        }

        self.characters[count].visible = visible;
        self.characters[count].line = self.cursor.line_number;
        {
            let justification = self.cursor.tags.justification;
            let line = &mut self.lines[self.cursor.line_number];

            if !matches!(code, unicode::LF | unicode::VT | unicode::CR) || line.character_count == 1
            {
                line.alignment = justification;
            }
        }

        let tags = &mut self.cursor.tags;
        let em_scale = self.em_scale;
        let spacing = font.style.normal_spacing_offset + advance.character_spacing;

        if code == unicode::TAB {
            let tab_size = font.face.tab_width * f32::from(font.style.tab_size) * scale;
            let tabs = (tags.x_advance / tab_size).ceil() * tab_size;

            tags.x_advance = if tabs > tags.x_advance {
                tabs
            } else {
                tags.x_advance + tab_size
            };
        } else if mono_spacing != 0.0 {
            tags.x_advance +=
                mono_spacing - mono_advance + (spacing * em_scale) + tags.character_spacing;
            if is_white_space || code == unicode::ZERO_WIDTH_SPACE {
                tags.x_advance += self.settings.word_spacing * em_scale;
            }
        } else {
            let lossy_scale = transform.map_or(1.0, |transform| transform.lossy_scale_x);

            tags.x_advance += (glyph.advance * lossy_scale + advance.adjustment.x_advance) * scale
                + (spacing + bold_spacing) * em_scale
                + tags.character_spacing;
            if is_white_space || code == unicode::ZERO_WIDTH_SPACE {
                tags.x_advance += self.settings.word_spacing * em_scale;
            }
        }

        self.characters[count].x_advance = tags.x_advance;
        if code == unicode::CR {
            tags.x_advance = tags.indent;
        }

        if matches!(
            code,
            unicode::LF
                | unicode::VT
                | unicode::END_OF_TEXT
                | unicode::LINE_SEPARATOR
                | unicode::PARAGRAPH_SEPARATOR
        ) || count == total - 1
        {
            self.end_line(padding, scale, advance);
            if matches!(
                code,
                unicode::LF
                    | unicode::VT
                    | unicode::HYPHEN
                    | unicode::LINE_SEPARATOR
                    | unicode::PARAGRAPH_SEPARATOR
            ) {
                self.break_line(code);

                return;
            }

            if code == unicode::END_OF_TEXT {
                self.index = self.source.tokens.len() as isize;
            }
        }

        if self.characters[count].visible {
            self.mesh.include(bottom_left, top_right);
        }

        if self.settings.word_wrapping {
            self.save_break_opportunity(code, is_white_space, is_first_character_of_line);
        }

        self.character_count += 1;
    }

    fn advance(&self, count: usize) -> Advance {
        let adjustment = if self.settings.kerning {
            self.source.kerning(self.fonts, count)
        } else {
            GlyphAdjustment::default()
        };

        Advance {
            adjustment,
            character_spacing: self.settings.character_spacing,
            bold_spacing: 0.0,
        }
    }

    fn wrap(&mut self, scale: f32, advance: Advance) {
        self.index = self.restore(&self.word_wrap.clone());
        if self.cursor.tags.line_height.is_some() {
            self.cursor.is_driven_line_spacing = true;
        }

        self.is_soft_hyphen_ignored = false;
        if self.characters[self.character_count].character == unicode::SOFT_HYPHEN {
            self.is_soft_hyphen_ignored = true;

            return;
        }

        let saved_soft_line_break = self.soft_line_break.index;

        if self.is_first_word_of_line
            && saved_soft_line_break != -1
            && saved_soft_line_break != self.last_soft_line_break
        {
            self.index = self.restore(&self.soft_line_break.clone());
            self.last_soft_line_break = saved_soft_line_break;
        }

        self.insert_new_line(scale, advance);
        self.is_start_of_new_line = true;
        self.is_first_word_of_line = true;
    }

    fn insert_new_line(&mut self, scale: f32, advance: Advance) {
        let count = self.character_count;
        let delta = self.cursor.max_line_ascender - self.cursor.start_of_line_ascender;

        if self.cursor.line_offset > 0.0
            && delta.abs() > 0.01
            && !self.cursor.is_driven_line_spacing
        {
            self.adjust_line_offset(self.cursor.first_character_of_line, count, delta);
            self.cursor.element_descender -= delta;
            self.cursor.line_offset += delta;
        }

        let line_ascender = self.cursor.max_line_ascender - self.cursor.line_offset;
        let line_descender = self.cursor.max_line_descender - self.cursor.line_offset;

        self.cursor.element_descender = self.cursor.element_descender.min(line_descender);
        self.max_visible_descender = self.cursor.element_descender;

        let last = count.saturating_sub(1);

        self.last_character_of_line = last;

        let (_, last_visible) = self.close_line_range(last);
        let font = self.fonts.font(self.cursor.current_font);
        let tags = &self.cursor.tags;
        let max_advance_offset = advance.adjustment.x_advance * scale
            + (font.style.normal_spacing_offset + advance.character_spacing + advance.bold_spacing)
                * self.em_scale
            - tags.character_spacing;
        let max_advance = self.characters[last_visible].x_advance - max_advance_offset;

        self.characters[last].x_advance = max_advance;

        let line = &mut self.lines[self.cursor.line_number];

        line.length = self.characters[last_visible].top_right.x;
        line.width = self.width_of_text_area;
        line.max_advance = max_advance;
        line.baseline = 0.0 - self.cursor.line_offset;
        line.ascender = line_ascender;
        line.descender = line_descender;
        line.line_height = line_ascender - line_descender + self.line_gap * self.base_scale;

        self.cursor.first_character_of_line = count;
        self.cursor.line_visible_character_count = 0;
        self.cursor.line_number += 1;
        self.ensure_line();

        let line_spacing = f64::from(self.settings.line_spacing) * f64::from(self.em_scale);

        if let Some(line_height) = self.cursor.tags.line_height {
            self.cursor.line_offset = mono_sum(
                self.cursor.line_offset,
                f64::from(line_height) + line_spacing,
            );
        } else {
            let ascender = self.characters[count].adjusted_ascender;
            let delta = self.line_offset_delta(ascender, line_spacing);

            self.cursor.line_offset += delta;
            self.cursor.start_of_line_ascender = ascender;
        }

        self.cursor.start_line();
        self.cursor.tags.x_advance = self.cursor.tags.indent;
    }

    fn close_line_range(&mut self, last: usize) -> (usize, usize) {
        let cursor = &mut self.cursor;
        let first = cursor.first_character_of_line;

        cursor.first_visible_character_of_line = first.max(cursor.first_visible_character_of_line);

        let first_visible = cursor.first_visible_character_of_line;

        if cursor.last_visible_character_of_line < first_visible {
            cursor.last_visible_character_of_line = first_visible;
        }

        let last_visible = cursor.last_visible_character_of_line;
        let line = &mut self.lines[cursor.line_number];

        line.first = first;
        line.first_visible = first_visible;
        line.last = last;
        line.last_visible = last_visible;
        line.character_count = last + 1 - first;
        line.visible_character_count = cursor.line_visible_character_count;

        (first_visible, last_visible)
    }

    fn end_line(&mut self, padding: f32, scale: f32, advance: Advance) {
        let count = self.character_count;
        let delta = self.cursor.max_line_ascender - self.cursor.start_of_line_ascender;

        if self.cursor.line_offset > 0.0
            && delta.abs() > 0.01
            && !self.cursor.is_driven_line_spacing
        {
            self.adjust_line_offset(self.cursor.first_character_of_line, count, delta);
            self.cursor.element_descender -= delta;
            self.cursor.line_offset += delta;
        }

        let line_ascender = self.cursor.max_line_ascender - self.cursor.line_offset;
        let line_descender = self.cursor.max_line_descender - self.cursor.line_offset;

        self.cursor.element_descender = self.cursor.element_descender.min(line_descender);
        self.max_visible_descender = self.cursor.element_descender;

        self.last_character_of_line = count;

        let (_, last_visible) = self.close_line_range(count);
        let font = self.fonts.font(self.cursor.current_font);
        let tags = &self.cursor.tags;
        let max_advance_offset =
            (font.style.normal_spacing_offset + advance.character_spacing + advance.bold_spacing)
                * self.em_scale
                - tags.character_spacing;
        let justification = tags.justification;
        let end = if self.characters[last_visible].visible {
            last_visible
        } else {
            self.last_character_of_line
        };
        let max_advance = self.characters[end].x_advance - max_advance_offset;
        let length = self.characters[last_visible].top_right.x - padding * scale;
        let line = &mut self.lines[self.cursor.line_number];

        line.length = length;
        line.width = self.width_of_text_area;
        if line.character_count == 1 {
            line.alignment = justification;
        }

        line.max_advance = max_advance;
        line.baseline = 0.0 - self.cursor.line_offset;
        line.ascender = line_ascender;
        line.descender = line_descender;
        line.line_height = line_ascender - line_descender + self.line_gap * self.base_scale;
    }

    fn break_line(&mut self, code: u32) {
        let count = self.character_count;

        self.cursor.line_number += 1;
        self.is_start_of_new_line = true;
        self.is_first_word_of_line = true;
        self.cursor.first_character_of_line = count + 1;
        self.cursor.line_visible_character_count = 0;
        self.ensure_line();

        let last_visible_ascender = self.characters[count].adjusted_ascender;
        let paragraph = if code == unicode::LF || code == unicode::PARAGRAPH_SEPARATOR {
            self.settings.paragraph_spacing
        } else {
            0.0
        };
        let spacing = (f64::from(self.settings.line_spacing) + f64::from(paragraph))
            * f64::from(self.em_scale);

        if let Some(line_height) = self.cursor.tags.line_height {
            self.cursor.line_offset =
                mono_sum(self.cursor.line_offset, f64::from(line_height) + spacing);
            self.cursor.is_driven_line_spacing = true;
        } else {
            let delta = self.line_offset_delta(last_visible_ascender, spacing);

            self.cursor.line_offset += delta;
            self.cursor.is_driven_line_spacing = false;
        }

        self.cursor.start_line();
        self.cursor.start_of_line_ascender = last_visible_ascender;

        let tags = &mut self.cursor.tags;

        tags.x_advance = 0.0 + tags.line_indent + tags.indent;
        self.word_wrap = self.save(self.index, count as isize);
        self.character_count += 1;
    }

    fn save_break_opportunity(
        &mut self,
        code: u32,
        is_white_space: bool,
        is_first_character_of_line: bool,
    ) {
        let count = self.character_count as isize;
        let non_breaking = self.cursor.tags.non_breaking;

        if (is_white_space
            || code == unicode::ZERO_WIDTH_SPACE
            || code == unicode::HYPHEN
            || code == unicode::SOFT_HYPHEN)
            && !non_breaking
            && !matches!(
                code,
                unicode::NO_BREAK_SPACE
                    | unicode::FIGURE_SPACE
                    | unicode::NON_BREAKING_HYPHEN
                    | unicode::NARROW_NO_BREAK_SPACE
                    | unicode::WORD_JOINER
            )
        {
            self.word_wrap = self.save(self.index, count);
            self.is_first_word_of_line = false;
            self.soft_line_break.index = -1;
        } else if !non_breaking && LineBreak::is_east_asian(code) {
            let is_leading = LineBreak::is_leading(code);
            let total = self.source.elements.len();
            let is_next_following = (self.character_count + 1) < total
                && LineBreak::is_following(
                    self.source.elements[self.character_count + 1].unicode(),
                );

            if !is_leading {
                if !is_next_following {
                    self.word_wrap = self.save(self.index, count);
                    self.is_first_word_of_line = false;
                }

                if self.is_first_word_of_line {
                    if is_white_space {
                        self.soft_line_break = self.save(self.index, count);
                    }

                    self.word_wrap = self.save(self.index, count);
                }
            } else if self.is_first_word_of_line && is_first_character_of_line {
                if is_white_space {
                    self.soft_line_break = self.save(self.index, count);
                }

                self.word_wrap = self.save(self.index, count);
            }
        } else if self.is_first_word_of_line {
            if is_white_space || (code == unicode::SOFT_HYPHEN && !self.is_soft_hyphen_ignored) {
                self.soft_line_break = self.save(self.index, count);
            }

            self.word_wrap = self.save(self.index, count);
        }
    }

    fn adjust_line_offset(&mut self, start: usize, end: usize, offset: f32) {
        for character in &mut self.characters[start..=end] {
            character.shift_vertically(offset);
        }
    }

    fn ensure_line(&mut self) {
        while self.lines.len() <= self.cursor.line_number {
            self.lines.push(LineInfo::new(self.settings.alignment));
        }
    }

    fn save(&self, index: isize, count: isize) -> WrapState {
        WrapState::new(
            index,
            count,
            &self.cursor,
            self.lines[self.cursor.line_number],
            self.mesh,
        )
    }

    fn restore(&mut self, state: &WrapState) -> isize {
        state.restore_into(&mut self.cursor);
        self.character_count = (state.count + 1) as usize;
        self.mesh = state.mesh;
        self.ensure_line();
        self.lines[self.cursor.line_number] = state.line;

        state.index
    }
}
