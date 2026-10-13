use text_mesh_core::{Alignment, LayoutChange, Length, LineHeight, Run, TextComponent};

use crate::css::Declarations;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LineState {
    margin_left: Option<Length>,
    margin_right: Option<Length>,
    alignments: Vec<Alignment>,
    line_height: Option<LineHeight>,
    width: Option<Length>,
    line_indent: Option<Length>,
    pub(crate) no_break: bool,
}

impl LineState {
    pub(crate) fn apply(&mut self, change: LayoutChange) {
        match change {
            LayoutChange::Margin { left, right } => {
                if left.is_some() {
                    self.margin_left = left;
                }

                if right.is_some() {
                    self.margin_right = right;
                }
            }
            LayoutChange::MarginEnd => {
                self.margin_left = None;
                self.margin_right = None;
            }
            LayoutChange::Align(alignment) => self.alignments.push(alignment),
            LayoutChange::AlignEnd => {
                self.alignments.pop();
            }
            LayoutChange::LineHeight(height) => self.line_height = Some(height),
            LayoutChange::LineHeightEnd => self.line_height = None,
            LayoutChange::Width(width) => self.width = Some(width),
            LayoutChange::WidthEnd => self.width = None,
            LayoutChange::LineIndent(indent) => self.line_indent = Some(indent),
            LayoutChange::LineIndentEnd => self.line_indent = None,
            LayoutChange::NoBreak(no_break) => self.no_break = no_break,
            LayoutChange::Indent(_) | LayoutChange::IndentEnd => {}
        }
    }

    pub(crate) fn of_line(&self, runs: &[Run]) -> Self {
        let mut state = self.clone();
        let mut at_last_visible = None;
        let mut at_last_character = None;

        for run in runs {
            match run {
                Run::Layout(change) => state.apply(*change),
                Run::Text { text, .. } => {
                    if text.chars().any(|character| !character.is_whitespace()) {
                        at_last_visible = Some(state.clone());
                    }

                    if !text.is_empty() {
                        at_last_character = Some(state.alignments.clone());
                    }
                }
                Run::Sprite { .. } => {
                    at_last_visible = Some(state.clone());
                    at_last_character = Some(state.alignments.clone());
                }
                Run::Position { .. } | Run::Space { .. } | Run::LineBreak => {}
            }
        }

        let visible = at_last_visible.unwrap_or_default();

        Self {
            margin_left: visible.margin_left,
            margin_right: visible.margin_right,
            width: visible.width,
            alignments: at_last_character.unwrap_or_else(|| state.alignments.clone()),
            ..state
        }
    }

    pub(crate) fn line_indent(&self) -> Option<Length> {
        self.line_indent
    }

    pub(crate) fn block(&self, component: &TextComponent) -> Option<String> {
        let mut declarations = Vec::new();
        let length = |length: Length| match length {
            Length::Pixels(pixels) => Declarations::em(pixels, component),
            Length::Percentage(percent) => format!("{percent}%"),
        };

        if let Some(left) = self.margin_left {
            declarations.push(format!("padding-left:{}", length(left)));
        }

        if let Some(right) = self.margin_right {
            declarations.push(format!("padding-right:{}", length(right)));
        }

        if let Some(&alignment) = self.alignments.last() {
            declarations.push(
                match alignment {
                    Alignment::Left => "text-align:left",
                    Alignment::Center => "text-align:center",
                    Alignment::Right => "text-align:right",
                    Alignment::Justified | Alignment::Flush => "text-align:justify",
                }
                .to_owned(),
            );
            if alignment == Alignment::Flush {
                declarations.push("text-align-last:justify".to_owned());
            }
        }

        match self.line_height {
            Some(LineHeight::Pixels(pixels)) => declarations.push(format!(
                "line-height:{}",
                Declarations::em(pixels, component)
            )),
            Some(LineHeight::Percentage { value, size, .. }) => declarations.push(format!(
                "line-height:{}",
                Declarations::em(size * value / 100.0, component)
            )),
            None => {}
        }

        if let Some(width) = self.width {
            declarations.push(format!("max-width:{}", length(width)));
        }

        (!declarations.is_empty()).then(|| declarations.join(";"))
    }
}
