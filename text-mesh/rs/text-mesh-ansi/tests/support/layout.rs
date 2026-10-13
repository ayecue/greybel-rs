use text_mesh_ansi::AnsiRenderer;
use text_mesh_core::{CellMetricContainer, Color, TextComponent};

use crate::support::ASSETS;

pub const GAME_CELLS: CellMetricContainer = CellMetricContainer {
    advance: 25.1875 / 42.0,
    line_height: 52.8125 / 42.0 + 0.045,
};

pub fn render_in_game_cells(text: &str) -> String {
    let component = TextComponent::new(Color::WHITE, 16.5, &ASSETS);

    AnsiRenderer::new(component, Color::BLACK)
        .with_cells(GAME_CELLS)
        .render(&component.parse(text))
}
