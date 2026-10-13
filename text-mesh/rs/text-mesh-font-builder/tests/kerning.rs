//! Tests kerning against fresh Unity bakes and TMP's existing LiberationSans assets.

use text_mesh_font::{GlyphAdjustment, RenderMode};
use text_mesh_test_support::font_file;

use crate::support::kerning::to_64ths;
use crate::support::{build_font, recordings};

#[test]
fn should_read_liberation_sans_kerning_pairs_like_text_mesh_pro() {
    let setup = text_mesh_test_support::setup();
    let bundled = setup
        .fonts
        .fonts
        .iter()
        .find(|font| font.name == "LiberationSans SDF")
        .expect("TMP's LiberationSans SDF");
    let recording = recordings()
        .into_iter()
        .find(|recording| recording.name == bundled.name)
        .expect("a LiberationSans recording");
    let data = font_file(&recording.source);

    let settings = recording
        .settings()
        .with_atlas(
            recording.atlas_width,
            recording.atlas_height,
            RenderMode::Smooth,
        )
        .with_kerning(true);
    let built_characters: Vec<char> = recording
        .glyphs
        .iter()
        .filter_map(|glyph| char::from_u32(glyph.unicode))
        .collect();
    let mut expected: Vec<(char, char, GlyphAdjustment, GlyphAdjustment)> = bundled
        .kerning
        .iter()
        .filter(|pair| {
            built_characters.contains(&pair.first) && built_characters.contains(&pair.second)
        })
        .map(|pair| (pair.first, pair.second, pair.pair.first, pair.pair.second))
        .collect();

    expected.sort_by_key(|&(first, second, _, _)| (first, second));

    let built = build_font(&data, settings);

    let actual: Vec<(char, char, GlyphAdjustment, GlyphAdjustment)> = built
        .asset
        .kerning
        .iter()
        .map(|pair| {
            (
                pair.first,
                pair.second,
                to_64ths(pair.pair.first),
                to_64ths(pair.pair.second),
            )
        })
        .collect();

    assert!(
        !expected.is_empty(),
        "TMP's LiberationSans SDF has kerning pairs"
    );
    assert_eq!(actual, expected);
}
