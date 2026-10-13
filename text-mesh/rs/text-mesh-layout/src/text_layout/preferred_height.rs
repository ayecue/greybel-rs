use text_mesh_font::{FontSet, GlyphAdjustment};

use crate::settings::TextSettings;
use crate::text_layout::cursor::{Cursor, WrapState};
use crate::text_layout::line_break::LineBreak;
use crate::text_layout::metrics::ElementMetrics;
use crate::text_layout::records::{CharacterInfo, Extents, LineInfo};
use crate::text_layout::source::{Source, Token};
use crate::text_layout::tags::TagContext;
use crate::text_layout::{mono_sum, unicode};

pub(crate) struct PreferredHeight<'a> {
    fonts: &'a FontSet,
    settings: &'a TextSettings,
    source: &'a Source,
    margin_width: f32,
    base_scale: f32,
    em_scale: f32,
    line_gap: f32,
    cursor: Cursor,
    characters: Vec<CharacterInfo>,
    index: isize,
    character_count: usize,
    is_first_word_of_line: bool,
    is_soft_hyphen_ignored: bool,
    is_last_character_east_asian: bool,
    rendered_height: f32,
    word_wrap: WrapState,
}

impl<'a> PreferredHeight<'a> {
    pub(crate) fn new(
        fonts: &'a FontSet,
        settings: &'a TextSettings,
        source: &'a Source,
        margin_width: f32,
    ) -> Self {
        let face = &fonts.font(fonts.primary).face;
        let cursor = Cursor::new(settings.alignment, fonts.primary);
        let initial = WrapState::new(
            0,
            0,
            &cursor,
            LineInfo::new(settings.alignment),
            Extents::EMPTY,
        );

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
            index: 0,
            character_count: 0,
            is_first_word_of_line: true,
            is_soft_hyphen_ignored: false,
            is_last_character_east_asian: false,
            rendered_height: 0.0,
            word_wrap: initial,
        }
    }

    pub(crate) fn compute(mut self) -> f32 {
        let tokens = &self.source.tokens;

        if tokens.is_empty() {
            return 0.0;
        }

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
                Token::CharacterSpacingEnd => {
                    let tags = &mut self.cursor.tags;

                    if self.character_count > 0 {
                        tags.x_advance -= tags.character_spacing;
                    }

                    tags.character_spacing = 0.0;
                }
            }

            self.index += 1;
        }

        let margin = &self.settings.margin;
        let mut height = self.rendered_height;

        height += if margin.top > 0.0 { margin.top } else { 0.0 };
        height += if margin.bottom > 0.0 {
            margin.bottom
        } else {
            0.0
        };

        ((f64::from(height) * 100.0 + 1.0) as i32) as f32 / 100.0
    }

    fn element(&mut self, element_index: usize) {
        let element = self.source.elements[element_index];
        let style = element.style;
        let total = self.source.elements.len();
        let count = self.character_count;
        let is_sprite = element.is_sprite();
        let metrics = ElementMetrics::new(self.fonts, &element, self.cursor.font_for(&element));
        let font = self.fonts.font(metrics.font);

        if !is_sprite {
            self.cursor.current_font = metrics.font;
            self.cursor.current_base_font = element.style_font.unwrap_or(self.fonts.primary);
        }

        self.cursor.tags.character_spacing = style.character_spacing;
        self.cursor.tags.mono_spacing = style.mono_spacing;

        let code = metrics.character;
        let mut scale = metrics.scale;
        let unmodified_scale = scale;

        if code == unicode::SOFT_HYPHEN || code == unicode::END_OF_TEXT {
            scale = 0.0;
        }

        self.characters[count].character = code;

        let glyph = element.glyph;
        let is_white_space = metrics.is_white_space();
        let adjustment = if self.settings.kerning {
            self.source.kerning(self.fonts, count)
        } else {
            GlyphAdjustment::default()
        };
        let character_spacing = self.settings.character_spacing;
        let mono_spacing = self.cursor.tags.mono_spacing;
        let mut mono_advance = 0.0;

        if mono_spacing != 0.0 {
            mono_advance = mono_spacing / 2.0 - (glyph.width / 2.0 + glyph.bearing_x) * scale;
            self.cursor.tags.x_advance += mono_advance;
        }

        let bold_spacing = if !is_sprite && style.bold {
            font.style.bold_spacing
        } else {
            0.0
        };

        let baseline_offset = metrics.baseline_offset;
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

        {
            let cursor = &mut self.cursor;

            if counts_for_line {
                if baseline_offset != 0.0 {
                    adjusted_ascender = ((element_ascender - baseline_offset)
                        / metrics.font_scale_multiplier)
                        .max(adjusted_ascender);
                    adjusted_descender = ((element_descender - baseline_offset)
                        / metrics.font_scale_multiplier)
                        .min(adjusted_descender);
                }

                cursor.max_line_ascender = adjusted_ascender.max(cursor.max_line_ascender);
                cursor.max_line_descender = adjusted_descender.min(cursor.max_line_descender);
            }

            let character = &mut self.characters[count];

            if counts_for_line {
                character.adjusted_ascender = adjusted_ascender;
                character.adjusted_descender = adjusted_descender;
                cursor.element_descender = element_descender - cursor.line_offset;
            } else {
                character.adjusted_ascender = cursor.max_line_ascender;
                character.adjusted_descender = cursor.max_line_descender;
                cursor.element_descender = cursor.max_line_descender - cursor.line_offset;
            }

            if cursor.line_number == 0 && counts_for_line {
                cursor.max_text_ascender = cursor.max_line_ascender;
            }
        }

        if code == unicode::TAB
            || (!is_white_space
                && code != unicode::ZERO_WIDTH_SPACE
                && code != unicode::SOFT_HYPHEN
                && code != unicode::END_OF_TEXT)
            || (code == unicode::SOFT_HYPHEN && !self.is_soft_hyphen_ignored)
            || is_sprite
        {
            let tags = &self.cursor.tags;
            let width_of_text_area = tags.width_of_text_area(self.margin_width);
            let width_scale = if code == unicode::SOFT_HYPHEN {
                unmodified_scale
            } else {
                scale
            };
            let text_width = tags.x_advance.abs() + glyph.advance * width_scale;
            let limit = if tags.is_justified_or_flush() {
                1.05
            } else {
                1.0
            };

            if text_width > width_of_text_area * limit
                && self.settings.word_wrapping
                && count != self.cursor.first_character_of_line
            {
                self.wrap();

                return;
            }
        }

        let em_scale = self.em_scale;
        let tags = &mut self.cursor.tags;
        let spacing = font.style.normal_spacing_offset + character_spacing;

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
            tags.x_advance += (glyph.advance + adjustment.x_advance) * scale
                + (spacing + bold_spacing) * em_scale
                + tags.character_spacing;
            if is_white_space || code == unicode::ZERO_WIDTH_SPACE {
                tags.x_advance += self.settings.word_spacing * em_scale;
            }
        }

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
            self.adjust_line_offset();

            let line_descender = self.cursor.max_line_descender - self.cursor.line_offset;

            self.cursor.element_descender = self.cursor.element_descender.min(line_descender);
            self.rendered_height = self.cursor.max_text_ascender - self.cursor.element_descender;
            if matches!(
                code,
                unicode::LF
                    | unicode::VT
                    | unicode::HYPHEN
                    | unicode::LINE_SEPARATOR
                    | unicode::PARAGRAPH_SEPARATOR
            ) {
                self.word_wrap = self.save(count as isize);
                self.cursor.line_number += 1;
                self.cursor.first_character_of_line = count + 1;

                let ascender = self.characters[count].adjusted_ascender;
                let paragraph = if code == unicode::LF || code == unicode::PARAGRAPH_SEPARATOR {
                    self.settings.paragraph_spacing
                } else {
                    0.0
                };

                self.advance_line(ascender, paragraph, true);

                let tags = &mut self.cursor.tags;

                tags.x_advance = 0.0 + tags.line_indent + tags.indent;
                self.character_count += 1;

                return;
            }

            if code == unicode::END_OF_TEXT {
                self.index = self.source.tokens.len() as isize;
            }
        }

        if self.settings.word_wrapping {
            self.save_break_opportunity(code, is_white_space);
        }

        self.character_count += 1;
    }

    fn wrap(&mut self) {
        let state = self.word_wrap.clone();

        state.restore_into(&mut self.cursor);
        self.character_count = (state.count + 1) as usize;
        self.index = state.index;
        self.is_soft_hyphen_ignored = false;
        if self.characters[self.character_count].character == unicode::SOFT_HYPHEN {
            self.is_soft_hyphen_ignored = true;

            return;
        }

        self.adjust_line_offset();

        let line_descender = self.cursor.max_line_descender - self.cursor.line_offset;

        self.cursor.element_descender = self.cursor.element_descender.min(line_descender);
        self.cursor.first_character_of_line = self.character_count;
        self.cursor.line_visible_character_count = 0;
        if self.settings.word_wrapping {
            self.rendered_height = self.cursor.max_text_ascender - self.cursor.element_descender;
        }

        self.cursor.line_number += 1;

        let ascender = self.characters[self.character_count].adjusted_ascender;

        self.advance_line(ascender, 0.0, false);
        self.cursor.tags.x_advance = self.cursor.tags.indent;
        self.is_first_word_of_line = true;
    }

    fn adjust_line_offset(&mut self) {
        let cursor = &mut self.cursor;
        let delta = cursor.max_line_ascender - cursor.start_of_line_ascender;

        if cursor.line_offset > 0.0 && delta.abs() > 0.01 && !cursor.is_driven_line_spacing {
            cursor.element_descender -= delta;
            cursor.line_offset += delta;
        }
    }

    /// Advances after a line break (`paragraph_break`) or wrap. TMP evaluates either offset in double precision
    /// like Mono, but only line breaks first store the offset in a float local.
    fn advance_line(&mut self, ascender: f32, paragraph: f32, paragraph_break: bool) {
        let spacing = (f64::from(self.settings.line_spacing) + f64::from(paragraph))
            * f64::from(self.em_scale);
        let delta = 0.0 - f64::from(self.cursor.max_line_descender)
            + f64::from(ascender)
            + f64::from(self.line_gap) * f64::from(self.base_scale)
            + spacing;
        let cursor = &mut self.cursor;

        if let Some(line_height) = cursor.tags.line_height {
            cursor.line_offset = mono_sum(cursor.line_offset, f64::from(line_height) + spacing);
            cursor.is_driven_line_spacing = true;
        } else if paragraph_break {
            cursor.line_offset += delta as f32;
            cursor.is_driven_line_spacing = false;
        } else {
            cursor.line_offset = mono_sum(cursor.line_offset, delta);
            cursor.is_driven_line_spacing = false;
        }

        cursor.start_line();
        cursor.start_of_line_ascender = ascender;
    }

    fn save_break_opportunity(&mut self, code: u32, is_white_space: bool) {
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
            self.word_wrap = self.save(count);
            self.is_first_word_of_line = false;
            self.is_last_character_east_asian = false;
        } else if !non_breaking && LineBreak::is_east_asian(code) {
            let is_leading = LineBreak::is_leading(code);
            let next = self.character_count + 1;
            let is_following = next < self.source.elements.len()
                && LineBreak::is_following(self.source.elements[next].unicode());

            if self.is_first_word_of_line || !is_leading {
                if !is_following {
                    self.word_wrap = self.save(count);
                    self.is_first_word_of_line = false;
                }

                if self.is_first_word_of_line {
                    self.word_wrap = self.save(count);
                }
            }

            self.is_last_character_east_asian = true;
        } else if self.is_last_character_east_asian {
            if !LineBreak::is_leading(code) {
                self.word_wrap = self.save(count);
            }

            self.is_last_character_east_asian = false;
        } else if self.is_first_word_of_line {
            self.word_wrap = self.save(count);
            self.is_last_character_east_asian = false;
        }
    }

    fn save(&self, count: isize) -> WrapState {
        WrapState::new(
            self.index,
            count,
            &self.cursor,
            LineInfo::new(self.settings.alignment),
            Extents::EMPTY,
        )
    }
}
