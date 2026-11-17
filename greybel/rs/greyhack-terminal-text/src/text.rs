use greyhack_text_mesh_assets::{TERMINAL_TEXT, TERMINAL_THEME, TEXT_ASSETS};
use text_mesh_ansi::AnsiRenderer;
use text_mesh_core::{LineSplitRule, TerminalOutput};
use text_mesh_layout::TextLayout;

/// Renders rich `text` as ANSI matching Grey Hack's terminal, with long lines split by `split_rule`. Positive
/// `columns` wraps lines to that many font cells; zero retains each line on one row. Non-default foreground colors
/// blend over the terminal background.
#[must_use]
pub fn ansi_text(text: &str, columns: u16, split_rule: LineSplitRule) -> String {
    let theme = &*TERMINAL_THEME;
    let renderer = AnsiRenderer::new(theme.component(&TEXT_ASSETS), theme.background)
        .with_cells(theme.cells)
        .with_split_rule(split_rule);

    if columns == 0 {
        return renderer.render_terminal(text);
    }

    let layout = TextLayout::new(
        &TERMINAL_TEXT.font_set,
        &TEXT_ASSETS,
        &TERMINAL_TEXT.settings,
    );

    renderer
        .laid_out(layout, columns.into())
        .render_terminal(text)
}

/// Removes rich-text markup and joins the visible lines, split by `split_rule`, with `\n`.
#[must_use]
pub fn plain_text(text: &str, split_rule: LineSplitRule) -> String {
    let component = TERMINAL_THEME.component(&TEXT_ASSETS);

    TerminalOutput::new(text)
        .with_split_rule(split_rule)
        .lines()
        .iter()
        .map(|line| component.parse(line).plain())
        .collect::<Vec<_>>()
        .join("\n")
}
