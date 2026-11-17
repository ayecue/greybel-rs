use text_mesh_core::{
    LayoutChange, Length, LineSplitRule, Run, TerminalOutput, Text, TextComponent,
};

use crate::css::Declarations;
use crate::escape::HtmlEscape;
use crate::line::LineState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HtmlRenderer<'a> {
    component: TextComponent<'a>,
    split_rule: LineSplitRule,
}

impl<'a> HtmlRenderer<'a> {
    #[must_use]
    pub const fn new(component: TextComponent<'a>) -> Self {
        Self {
            component,
            split_rule: LineSplitRule::Game,
        }
    }

    #[must_use]
    pub const fn with_split_rule(self, split_rule: LineSplitRule) -> Self {
        Self { split_rule, ..self }
    }

    #[must_use]
    pub fn render(&self, text: &Text) -> String {
        let mut out = String::new();
        let mut previous_inline = None;
        let mut state = LineState::default();

        for (number, line) in text
            .runs
            .split(|run| matches!(run, Run::LineBreak))
            .enumerate()
        {
            let block = state.of_line(line).block(&self.component);
            let positioned = line.iter().any(Self::starts_segment);
            let inline = !positioned && block.is_none();

            if previous_inline == Some(true) && inline {
                out.push_str("<br>");
            }

            if let Some(block) = &block {
                out.push_str("<span style=\"display:block;");
                out.push_str(block);
                out.push_str("\">");
            }

            if number > 0
                && let Some(indent) = state.line_indent()
            {
                self.spacer(indent, &mut out);
            }

            if positioned {
                self.render_grid_line(line, &mut state, &mut out);
            } else {
                self.flow(line, &mut state, &mut out);
            }

            if block.is_some() {
                out.push_str("</span>");
            }

            previous_inline = Some(inline);
        }

        out
    }

    fn starts_segment(run: &Run) -> bool {
        matches!(
            run,
            Run::Position { .. } | Run::Layout(LayoutChange::Indent(_))
        )
    }

    fn render_grid_line(&self, line: &[Run], state: &mut LineState, out: &mut String) {
        out.push_str("<span style=\"display:grid;align-items:baseline\">");

        let mut start = 0;
        let mut offset = String::from("0");

        for (index, run) in line.iter().enumerate() {
            let x = match run {
                Run::Position { x } | Run::Layout(LayoutChange::Indent(x)) => *x,
                _ => continue,
            };

            self.segment(&line[start..index], &offset, state, out);
            offset = self.length(x);
            start = index + 1;
        }

        self.segment(&line[start..], &offset, state, out);
        out.push_str("</span>");
    }

    fn segment(&self, runs: &[Run], offset: &str, state: &mut LineState, out: &mut String) {
        if runs.is_empty() {
            return;
        }

        out.push_str("<span style=\"grid-area:1/1;justify-self:start;margin-left:");
        out.push_str(offset);
        out.push_str("\">");
        self.flow(runs, state, out);
        out.push_str("</span>");
    }

    fn length(&self, length: Length) -> String {
        match length {
            Length::Pixels(pixels) => Declarations::em(pixels, &self.component),
            Length::Percentage(percent) => format!("{percent}%"),
        }
    }

    fn spacer(&self, width: Length, out: &mut String) {
        out.push_str("<span style=\"display:inline-block;width:");
        out.push_str(&self.length(width));
        out.push_str("\"></span>");
    }

    fn flow(&self, runs: &[Run], state: &mut LineState, out: &mut String) {
        if state.no_break {
            out.push_str("<span style=\"white-space:nowrap\">");
        }

        for run in runs {
            match run {
                Run::Text { text, style } => {
                    let declarations = Declarations::new(style, &self.component);

                    if !declarations.is_empty() {
                        out.push_str("<span style=\"");
                        out.push_str(declarations.as_str());
                        out.push_str("\">");
                    }

                    match Declarations::transform(style) {
                        Some((outer, inner)) => {
                            for character in text.chars() {
                                if let Some(outer) = &outer {
                                    out.push_str("<span style=\"");
                                    out.push_str(outer);
                                    out.push_str("\">");
                                }

                                out.push_str("<span style=\"");
                                out.push_str(&inner);
                                out.push_str("\">");
                                HtmlEscape::push(character.encode_utf8(&mut [0; 4]), out);
                                out.push_str("</span>");
                                if outer.is_some() {
                                    out.push_str("</span>");
                                }
                            }
                        }
                        None => HtmlEscape::push(text, out),
                    }

                    if !declarations.is_empty() {
                        out.push_str("</span>");
                    }
                }
                Run::Sprite {
                    index,
                    color,
                    style,
                } => {
                    out.push_str(&format!(
                        "<span class=\"sprite\" data-index=\"{index}\" style=\"{}\"></span>",
                        Declarations::sprite(*color, style, &self.component).as_str()
                    ));
                }
                Run::Space { width } => self.spacer(Length::Pixels(*width), out),
                Run::Layout(change) => {
                    let no_break = state.no_break;

                    state.apply(*change);
                    if let LayoutChange::LineIndent(indent) = change {
                        self.spacer(*indent, out);
                    }

                    if state.no_break != no_break {
                        out.push_str(if state.no_break {
                            "<span style=\"white-space:nowrap\">"
                        } else {
                            "</span>"
                        });
                    }
                }
                Run::Position { .. } | Run::LineBreak => {}
            }
        }

        if state.no_break {
            out.push_str("</span>");
        }
    }

    #[must_use]
    pub fn render_terminal(&self, output: &str) -> String {
        TerminalOutput::new(output)
            .with_split_rule(self.split_rule)
            .lines()
            .iter()
            .map(|line| {
                format!(
                    "<div class=\"line\">{}</div>",
                    self.render(&self.component.parse(line))
                )
            })
            .collect()
    }
}
