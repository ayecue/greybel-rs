//! Every render mode builds; only the SDF modes the test setup uses have Unity recordings (see `unity.rs`), the others are checked for shape.

use text_mesh_font::RenderMode;
use text_mesh_font_builder::FontBuildSettings;
use text_mesh_test_support::font_file;

use crate::support::build_font;
use crate::support::render_modes::MODES;

#[test]
fn should_build_every_render_mode() {
    let data = font_file("FiraMono-Regular.ttf");
    let settings = FontBuildSettings::new("Fira Mono", "Ag?".chars()).with_sampling_size(24, 4);

    let built: Vec<_> = MODES
        .iter()
        .map(|&mode| {
            let settings = settings.clone().with_atlas(128, 128, mode);
            let font = build_font(&data, settings);

            (mode, font)
        })
        .collect();

    for (mode, font) in built {
        assert_eq!(font.asset.glyphs.len(), 3, "{mode:?}");
        assert_eq!(font.atlases.len(), 1, "{mode:?}");
        assert!(
            font.atlases[0].pixels.iter().any(|&alpha| alpha > 128),
            "{mode:?}"
        );
        assert_eq!(font.asset.atlas.map(|atlas| atlas.render_mode), Some(mode));
        assert_eq!(
            font.asset.material.map(|material| material.gradient_scale),
            mode.is_distance_field().then_some(5.0),
            "{mode:?}"
        );
        for glyph in font.asset.glyphs.iter() {
            assert!(glyph.metrics.advance > 0.0, "{mode:?} {glyph:?}");
            assert!(
                glyph.rect.width > 0 && glyph.rect.height > 0,
                "{mode:?} {glyph:?}"
            );
        }
    }
}

#[test]
fn should_grid_fit_hinted_metrics() {
    let data = font_file("FiraMono-Regular.ttf");
    let settings = FontBuildSettings::new("Fira Mono", "Ag".chars())
        .with_sampling_size(24, 4)
        .with_atlas(128, 128, RenderMode::HintedSmooth);

    let font = build_font(&data, settings);

    for glyph in font.asset.glyphs.iter() {
        let metrics = glyph.metrics;

        for value in [
            metrics.width,
            metrics.height,
            metrics.bearing_x,
            metrics.bearing_y,
            metrics.advance,
        ] {
            assert_eq!(value.fract(), 0.0, "{glyph:?}");
        }
    }
}
