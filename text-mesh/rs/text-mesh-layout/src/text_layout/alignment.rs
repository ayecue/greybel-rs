use text_mesh_core::{Alignment, Style};

use crate::layout::{
    Element, ElementGeometry, ElementKind, Layout, LayoutGeometry, Line, LineGeometry, Position,
    Quad,
};
use crate::settings::{TextSettings, VerticalAlignment};
use crate::text_layout::generator::GeneratedText;
use crate::text_layout::line_break::LineBreak;
use crate::text_layout::records::{CharacterInfo, LineInfo, Point};
use crate::text_layout::source::{Source, SourceElement};
use crate::text_layout::unicode;

#[derive(Debug, Clone, Copy, PartialEq)]
struct ItemRect {
    x_min: f32,
    y_min: f32,
    y_max: f32,
}

impl ItemRect {
    fn new(width: f32, height: f32) -> Self {
        let x_min = -0.5 * width;
        let y_min = -0.5 * height;

        Self {
            x_min,
            y_min,
            y_max: y_min + height,
        }
    }

    fn x(&self, x: f32) -> f32 {
        x - self.x_min
    }

    fn y(&self, y: f32) -> f32 {
        self.y_max - y
    }
}

pub(crate) struct Aligner<'a> {
    settings: &'a TextSettings,
    source: &'a Source,
    text: GeneratedText,
    rect: ItemRect,
    width: f32,
    height: f32,
}

impl<'a> Aligner<'a> {
    pub(crate) fn new(
        settings: &'a TextSettings,
        source: &'a Source,
        text: GeneratedText,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            settings,
            source,
            text,
            rect: ItemRect::new(width, height),
            width,
            height,
        }
    }

    pub(crate) fn layout(mut self, preferred_height: f32) -> (Layout, LayoutGeometry) {
        let anchor = self.anchor();
        let count = self.text.characters.len();
        let mut justification = Point::default();
        let mut last_line = 0;
        let mut is_first_separator = false;

        for index in 0..count {
            let character = self.text.characters[index];
            let current_line = character.line;
            let line = self.text.lines[current_line];

            self.justify(
                &mut justification,
                &mut is_first_separator,
                index,
                &character,
                &line,
                current_line != last_line,
            );

            let offset = anchor + justification;

            self.text.characters[index].translate(offset);
            if current_line != last_line {
                self.text.lines[last_line].translate(offset);
            }

            if index == count - 1 {
                self.text.lines[current_line].translate(offset);
            }

            last_line = current_line;
        }

        let line_count = self.text.characters.last().map_or(0, |last| last.line + 1);
        let characters = self.text.characters.iter().zip(&self.source.elements);
        let lines = &self.text.lines[..line_count];
        let layout = Layout {
            width: self.width,
            height: self.height,
            preferred_height,
            elements: characters
                .clone()
                .map(|(character, element)| self.element(character, element.kind, element.style))
                .collect(),
            lines: lines.iter().map(|line| self.line(line)).collect(),
        };
        let geometry = LayoutGeometry {
            elements: characters
                .map(|(character, element)| self.element_geometry(character, element))
                .collect(),
            lines: lines
                .iter()
                .map(|line| LineGeometry {
                    first_visible: line.first_visible,
                    last_visible: line.last_visible,
                })
                .collect(),
        };

        (layout, geometry)
    }

    fn anchor(&self) -> Point {
        let margin = &self.settings.margin;
        let text = &self.text;
        let rect = &self.rect;
        let middle = (rect.y_min + rect.y_max) / 2.0;
        let x = rect.x_min + margin.left;
        let y = match self.settings.vertical_alignment {
            VerticalAlignment::Top => rect.y_max + (0.0 - text.max_text_ascender - margin.top),
            VerticalAlignment::Middle => {
                middle
                    + (0.0
                        - (text.max_text_ascender + margin.top + text.max_visible_descender
                            - margin.bottom)
                            / 2.0)
            }
            VerticalAlignment::Bottom => {
                rect.y_min + (0.0 - text.max_visible_descender + margin.bottom)
            }
            VerticalAlignment::Baseline => middle,
            VerticalAlignment::Geometry => {
                middle
                    + (0.0 - (text.mesh.max.y + margin.top + text.mesh.min.y - margin.bottom) / 2.0)
            }
            VerticalAlignment::Capline => {
                middle + (0.0 - (text.max_cap_height - margin.top - margin.bottom) / 2.0)
            }
        };

        Point::new(x, y)
    }

    fn justify(
        &self,
        justification: &mut Point,
        is_first_separator: &mut bool,
        index: usize,
        character: &CharacterInfo,
        line: &LineInfo,
        is_new_line: bool,
    ) {
        let unicode = character.character;

        match line.alignment {
            Alignment::Left => *justification = Point::new(line.margin_left, 0.0),
            Alignment::Center => {
                *justification = Point::new(
                    line.margin_left + line.width / 2.0 - line.max_advance / 2.0,
                    0.0,
                );
            }
            Alignment::Right => {
                *justification = Point::new(line.margin_left + line.width - line.max_advance, 0.0);
            }
            Alignment::Justified | Alignment::Flush => {
                if matches!(
                    unicode,
                    0x0A | unicode::SOFT_HYPHEN
                        | unicode::ZERO_WIDTH_SPACE
                        | unicode::WORD_JOINER
                        | unicode::END_OF_TEXT
                ) {
                    return;
                }

                let last = self.text.characters[line.last];
                let is_flush = line.alignment == Alignment::Flush;

                if !LineBreak::is_control(last.character) && character.line < self.text.line_number
                    || is_flush
                    || line.max_advance > line.width
                {
                    if is_new_line || index == 0 {
                        *justification = Point::new(line.margin_left, 0.0);
                        *is_first_separator = LineBreak::is_separator(unicode);
                    } else {
                        let gap = line.width - line.max_advance;
                        let mut visible_count =
                            line.visible_character_count as i32 - 1 + line.control_character_count;
                        let mut spaces = if last.visible {
                            line.whitespace_count
                        } else {
                            line.whitespace_count - 1
                        } - line.control_character_count;

                        if *is_first_separator {
                            spaces -= 1;
                            visible_count += 1;
                        }

                        let ratio = if spaces > 0 {
                            self.settings.word_wrapping_ratios
                        } else {
                            1.0
                        };

                        if spaces < 1 {
                            spaces = 1;
                        }

                        if unicode != unicode::NO_BREAK_SPACE
                            && (unicode == unicode::TAB || LineBreak::is_separator(unicode))
                        {
                            justification.x += gap * (1.0 - ratio) / spaces as f32;
                        } else {
                            justification.x += gap * ratio / visible_count as f32;
                        }
                    }
                } else {
                    *justification = Point::new(line.margin_left, 0.0);
                }
            }
        }
    }

    fn element(&self, character: &CharacterInfo, kind: ElementKind, style: Style) -> Element {
        let rect = &self.rect;

        Element {
            kind,
            style,
            line: character.line,
            visible: character.visible,
            origin: rect.x(character.origin),
            advance: rect.x(character.x_advance),
            baseline: rect.y(character.baseline),
            ascender: rect.y(character.ascender),
            descender: rect.y(character.descender),
            left: rect.x(character.top_left.x),
            top: rect.y(character.top_left.y),
            right: rect.x(character.bottom_right.x),
            bottom: rect.y(character.bottom_right.y),
            point_size: character.point_size,
            scale: character.scale,
        }
    }

    fn element_geometry(
        &self,
        character: &CharacterInfo,
        element: &SourceElement,
    ) -> ElementGeometry {
        ElementGeometry {
            quad: Quad {
                bottom_left: self.position(character.bottom_left),
                top_left: self.position(character.top_left),
                top_right: self.position(character.top_right),
                bottom_right: self.position(character.bottom_right),
            },
            font: character.font,
            base_font: element.style_font,
            padding: character.padding,
            style_padding: character.style_padding,
        }
    }

    fn position(&self, point: Point) -> Position {
        Position {
            x: self.rect.x(point.x),
            y: self.rect.y(point.y),
        }
    }

    fn line(&self, line: &LineInfo) -> Line {
        let rect = &self.rect;

        Line {
            first: line.first,
            last: line.last,
            ascender: rect.y(line.ascender),
            descender: rect.y(line.descender),
            baseline: rect.y(line.baseline),
            line_height: line.line_height,
            width: line.width,
            length: line.length,
            margin_left: line.margin_left,
            margin_right: line.margin_right,
            alignment: line.alignment,
        }
    }
}
