use text_mesh_core::{Alignment, LayoutChange, Length, LineHeight};
use text_mesh_font::FontSet;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValueStack<T: Copy> {
    items: Vec<T>,
}

impl<T: Copy> ValueStack<T> {
    pub(crate) fn new(default: T) -> Self {
        Self {
            items: vec![default],
        }
    }

    pub(crate) fn add(&mut self, item: T) {
        self.items.push(item);
    }

    pub(crate) fn remove(&mut self) -> T {
        if self.items.len() > 1 {
            self.items.pop();
        }

        self.items[self.items.len() - 1]
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TagContext<'a> {
    pub fonts: &'a FontSet,
    pub margin_width: f32,
    pub current_font: usize,
    pub tag_font: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TagState {
    pub x_advance: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub width: Option<f32>,
    pub indent: f32,
    pub indents: ValueStack<f32>,
    pub line_indent: f32,
    pub line_height: Option<f32>,
    pub justification: Alignment,
    pub justifications: ValueStack<Alignment>,
    pub non_breaking: bool,
    pub character_spacing: f32,
    pub mono_spacing: f32,
}

impl TagState {
    pub(crate) fn new(alignment: Alignment) -> Self {
        Self {
            x_advance: 0.0,
            margin_left: 0.0,
            margin_right: 0.0,
            width: None,
            indent: 0.0,
            indents: ValueStack::new(0.0),
            line_indent: 0.0,
            line_height: None,
            justification: alignment,
            justifications: ValueStack::new(alignment),
            non_breaking: false,
            character_spacing: 0.0,
            mono_spacing: 0.0,
        }
    }

    pub(crate) fn width_of_text_area(&self, margin_width: f32) -> f32 {
        let area = margin_width + 0.0001 - self.margin_left - self.margin_right;

        self.width.map_or(area, |width| area.min(width))
    }

    pub(crate) fn is_justified_or_flush(&self) -> bool {
        matches!(self.justification, Alignment::Justified | Alignment::Flush)
    }

    pub(crate) fn position(&mut self, x: Length, margin_width: f32) {
        self.x_advance = Self::resolve(x, margin_width);
    }

    pub(crate) fn add_space(&mut self, width: f32) {
        self.x_advance += width;
    }

    pub(crate) fn apply(&mut self, change: &LayoutChange, context: &TagContext) {
        let margin_width = context.margin_width;

        match *change {
            LayoutChange::Indent(length) => {
                self.indent = Self::resolve(length, margin_width);
                self.indents.add(self.indent);
                self.x_advance = self.indent;
            }
            LayoutChange::IndentEnd => self.indent = self.indents.remove(),
            LayoutChange::LineIndent(length) => {
                self.line_indent = Self::resolve(length, margin_width);
                self.x_advance += self.line_indent;
            }
            LayoutChange::LineIndentEnd => self.line_indent = 0.0,
            LayoutChange::Margin { left, right } => {
                let width = self.width.unwrap_or(0.0);
                let margin = |length| match length {
                    Length::Pixels(value) => value,
                    Length::Percentage(value) => (margin_width - width) * value / 100.0,
                };

                if let Some(left) = left {
                    let value: f32 = margin(left);

                    self.margin_left = if value >= 0.0 { value } else { 0.0 };
                }

                if let Some(right) = right {
                    let value: f32 = margin(right);

                    self.margin_right = if value >= 0.0 { value } else { 0.0 };
                }
            }
            LayoutChange::MarginEnd => {
                self.margin_left = 0.0;
                self.margin_right = 0.0;
            }
            LayoutChange::LineHeight(height) => {
                self.line_height = Some(match height {
                    LineHeight::Pixels(value) => value,
                    LineHeight::Percentage { value, size, .. } => {
                        let fonts = context.fonts;
                        let current = fonts.font(context.tag_font.unwrap_or(context.current_font));
                        let scale = size / current.face.point_size * current.face.scale;

                        fonts.font(fonts.primary).face.line_height * value / 100.0 * scale
                    }
                });
            }
            LayoutChange::LineHeightEnd => self.line_height = None,
            LayoutChange::Width(length) => self.width = Some(Self::resolve(length, margin_width)),
            LayoutChange::WidthEnd => self.width = None,
            LayoutChange::Align(alignment) => {
                self.justification = alignment;
                self.justifications.add(alignment);
            }
            LayoutChange::AlignEnd => self.justification = self.justifications.remove(),
            LayoutChange::NoBreak(is_enabled) => self.non_breaking = is_enabled,
        }
    }

    fn resolve(length: Length, margin_width: f32) -> f32 {
        match length {
            Length::Pixels(value) => value,
            Length::Percentage(value) => margin_width * value / 100.0,
        }
    }
}

#[cfg(test)]
#[path = "tests/tags.rs"]
mod tests;
