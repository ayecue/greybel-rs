use greyhack_text_mesh_assets::{FontFile, TERMINAL_THEME, TEXT_ASSETS};
use support::assets::parse;
use text_mesh_core::{Color, Run};

use crate::support;

#[test]
fn should_style_text_with_the_games_style_sheet() {
    let text = "<style=C1>x";
    let runs = parse(text);

    assert!(
        matches!(&runs[..], [Run::Text { text, style }] if text == "x" && style.color == Color::new(255, 255, 64, 255))
    );
}

#[test]
fn should_show_the_terminal_caret_sprites() {
    let text = "<sprite index=0 color=#00F0DD><sprite index=2><sprite index=3>";
    let runs = parse(text);
    let indices: Vec<u32> = runs
        .iter()
        .filter_map(|run| match run {
            Run::Sprite { index, .. } => Some(*index),
            _ => None,
        })
        .collect();

    assert_eq!(indices, [0, 2]);
}

#[test]
fn should_load_the_fonts_and_materials_of_the_games_resources() {
    let text = "<font=\"LiberationSans SDF\" material=\"LiberationSans SDF - Outline\">a<material=\"SourceCodePro-Regular SDF\">b";
    let runs = parse(text);
    let fonts: Vec<(Option<u16>, Option<u16>)> = runs
        .iter()
        .filter_map(|run| match run {
            Run::Text { style, .. } => Some((style.font, style.material)),
            _ => None,
        })
        .collect();

    assert_eq!(fonts, [(Some(0), Some(1)), (Some(0), Some(2))]);
}

#[test]
fn should_use_the_games_default_terminal_theme() {
    let theme = *TERMINAL_THEME;
    let component = theme.component(&TEXT_ASSETS);

    assert_eq!(
        (component.color, theme.background, component.size),
        (
            Color::new(0, 240, 221, 255),
            Color::new(4, 8, 26, 255),
            16.5
        )
    );
}

#[test]
fn should_have_a_font_file_with_its_license_for_every_font_asset_text_can_load() {
    let assets = &TEXT_ASSETS.fonts;
    let files: Vec<Option<&FontFile>> = assets.iter().map(|asset| FontFile::find(asset)).collect();

    assert!(files.iter().all(|file| file.is_some_and(|file| {
        file.data.starts_with(&[0, 1, 0, 0]) && file.license.contains("SIL Open Font License")
    })));
}

#[test]
fn should_measure_terminal_cells_from_the_terminal_font() {
    let theme = *TERMINAL_THEME;
    let cells = theme.cells;

    assert_eq!(
        (
            format!("{:.4}", cells.advance),
            format!("{:.4}", cells.line_height)
        ),
        ("0.5997".to_owned(), "1.3024".to_owned())
    );
}

#[test]
fn should_have_a_font_file_for_the_terminal_font() {
    let theme = *TERMINAL_THEME;
    let file = FontFile::find(theme.font);

    assert_eq!(file.map(|file| file.family), Some("Source Code Pro"));
}
