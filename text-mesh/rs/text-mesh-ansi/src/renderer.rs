use text_mesh_core::{
    CellMetricContainer, Color, LayoutChange, Length, LineSplitRule, Run, Style, TerminalOutput,
    Text, TextComponent,
};
use text_mesh_layout::{ElementKind, Layout, TextLayout};

use crate::grid::{Cell, Grid, QuarterTurn};
use crate::sgr::Sgr;

#[derive(Debug, Clone, Copy)]
pub struct AnsiRenderer<'a> {
    component: TextComponent<'a>,
    background: Color,
    paints_background: bool,
    cells: CellMetricContainer,
    layout: Option<(TextLayout<'a>, usize)>,
    split_rule: LineSplitRule,
}

impl<'a> AnsiRenderer<'a> {
    const SOFT_HYPHEN: char = '\u{ad}';
    const SPRITE: char = '█';

    #[must_use]
    pub const fn new(component: TextComponent<'a>, background: Color) -> Self {
        Self {
            component,
            background,
            paints_background: false,
            cells: CellMetricContainer::MONOSPACE,
            layout: None,
            split_rule: LineSplitRule::Game,
        }
    }

    #[must_use]
    pub const fn with_split_rule(self, split_rule: LineSplitRule) -> Self {
        Self { split_rule, ..self }
    }

    #[must_use]
    pub const fn with_painted_background(self) -> Self {
        Self {
            paints_background: true,
            ..self
        }
    }

    #[must_use]
    pub const fn with_cells(self, cells: CellMetricContainer) -> Self {
        Self { cells, ..self }
    }

    #[must_use]
    pub const fn laid_out(self, layout: TextLayout<'a>, columns: usize) -> Self {
        Self {
            layout: Some((layout, columns)),
            ..self
        }
    }

    #[must_use]
    pub fn render(&self, text: &Text) -> String {
        match self.layout {
            Some((layout, columns)) => {
                #[allow(clippy::cast_precision_loss)]
                let width = columns as f32 * self.cells.advance * self.component.size;
                let height = layout.preferred_height(text, width);

                self.render_layout(&layout.layout(text, width, height))
            }
            None => self.render_runs(text),
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn render_layout(&self, layout: &Layout) -> String {
        let cell_width = self.cells.advance * self.component.size;
        let row_height = self.cells.line_height * self.component.size;
        let first = layout.lines.first().map_or(0.0, |line| line.baseline);
        let mut grid = Grid::default();

        for line in &layout.lines {
            grid.ensure_row(((line.baseline - first) / row_height).round() as i32);
        }

        let mut previous: Option<(usize, usize, f32)> = None;
        let mut sprites = Vec::new();

        for element in &layout.elements {
            let row = ((element.baseline - first) / row_height).round() as i32;
            let column = match previous {
                Some((line, column, advance))
                    if line == element.line && (element.origin - advance).abs() < 0.01 =>
                {
                    column + 1
                }
                _ => (element.origin / cell_width).round().max(0.0) as usize,
            };

            if matches!(element.kind, ElementKind::Sprite { .. })
                && (element.advance - element.origin).abs() < 0.01
            {
                sprites.push((row, column, element));
                continue;
            }

            previous = Some((element.line, column, element.advance));

            let style = &element.style;

            match element.kind {
                ElementKind::Glyph { character, .. } => {
                    if character == Self::SOFT_HYPHEN || matches!(character, '\n' | '\r' | '\u{b}')
                    {
                        continue;
                    }

                    let decorated = style.highlight.is_some()
                        || style.underline.is_some()
                        || style.strikethrough.is_some();

                    if !element.visible && !decorated {
                        continue;
                    }

                    let character = if element.visible {
                        QuarterTurn::apply(character, style.rotation)
                    } else {
                        ' '
                    };

                    grid.put(
                        row,
                        column,
                        Cell {
                            character,
                            sgr: self.sgr(style),
                        },
                        true,
                    );
                }
                ElementKind::Sprite { color, .. } => {
                    let style = Style { color, ..*style };
                    let cell = Cell {
                        character: Self::SPRITE,
                        sgr: self.sgr(&style),
                    };

                    grid.put(row, column, cell, true);
                }
            }
        }

        for (row, column, element) in sprites {
            if let ElementKind::Sprite { color, .. } = element.kind {
                let style = Style {
                    color,
                    ..element.style
                };
                let cell = Cell {
                    character: Self::SPRITE,
                    sgr: self.sgr(&style),
                };

                grid.put(row, column, cell, true);
            }
        }

        let mut out = String::new();

        grid.write(self.blank(), &mut out);

        out
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn render_runs(&self, text: &Text) -> String {
        let cell_width = self.cells.advance * self.component.size;
        let row_height = self.cells.line_height * self.component.size;
        let column = |x: f32| (x / cell_width).round().max(0.0) as usize;
        let row_of =
            |line: i32, style: &Style| line + (-style.vertical_offset / row_height).round() as i32;
        let mut grid = Grid::default();
        let mut x = 0.0_f32;
        let mut line = 0;
        let mut lowest = 0;

        grid.ensure_row(0);
        for run in &text.runs {
            match run {
                Run::Text { text, style } => {
                    let sgr = self.sgr(style);
                    let row = row_of(line, style);

                    lowest = lowest.max(row);

                    let advance = if style.mono_spacing == 0.0 {
                        cell_width
                    } else {
                        style.mono_spacing
                    } + style.character_spacing;

                    for character in text
                        .chars()
                        .filter(|&character| character != Self::SOFT_HYPHEN)
                    {
                        let character = QuarterTurn::apply(character, style.rotation);
                        let draws = character != ' '
                            || style.highlight.is_some()
                            || style.underline.is_some()
                            || style.strikethrough.is_some();

                        grid.put(row, column(x), Cell { character, sgr }, draws);
                        x += advance;
                    }
                }
                Run::Sprite { color, style, .. } => {
                    let row = row_of(line, style);

                    lowest = lowest.max(row);

                    let style = Style {
                        color: *color,
                        ..*style
                    };
                    let cell = Cell {
                        character: Self::SPRITE,
                        sgr: self.sgr(&style),
                    };

                    grid.put(row, column(x), cell, true);
                }
                Run::Position {
                    x: Length::Pixels(position),
                } => x = *position,
                Run::Position { .. } => {}
                Run::Space { width } => x += width,
                Run::Layout(LayoutChange::Indent(Length::Pixels(indent))) => x = *indent,
                Run::Layout(LayoutChange::LineIndent(Length::Pixels(indent))) => x += indent,
                Run::Layout(_) => {}
                Run::LineBreak => {
                    line = lowest + 1;
                    lowest = line;
                    x = 0.0;
                    grid.ensure_row(line);
                }
            }
        }

        let mut out = String::new();

        grid.write(self.blank(), &mut out);

        out
    }

    fn sgr(&self, style: &Style) -> Sgr {
        Sgr::new(
            style,
            self.component.color,
            self.background,
            self.paints_background,
        )
    }

    fn blank(&self) -> Sgr {
        Sgr::blank(
            self.component.color,
            self.background,
            self.paints_background,
        )
    }

    #[must_use]
    pub fn render_terminal(&self, output: &str) -> String {
        TerminalOutput::new(output)
            .with_split_rule(self.split_rule)
            .lines()
            .iter()
            .map(|line| self.render(&self.component.parse(line)))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
