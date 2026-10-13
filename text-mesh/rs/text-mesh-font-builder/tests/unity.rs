//! Builds every font Unity baked with its settings and compares face info, metrics, atlas rectangles and pixels.

use crate::support::comparison::{BuiltRecording, built_recordings};
use crate::support::unity::{KNOWN_DIFFERENCES, KnownDifference};

#[test]
fn should_build_every_recorded_font() {
    let expected = 3;

    let built = built_recordings();

    assert_eq!(built.len(), expected);
    for known in KNOWN_DIFFERENCES {
        assert!(
            built.iter().any(|font| font.name() == known.font),
            "{}: no such recording",
            known.font
        );
    }
}

#[test]
fn should_compute_unitys_face_info() {
    let built = built_recordings();

    let differences: Vec<_> = built
        .iter()
        .map(|font| (font.name(), font.face_differences()))
        .filter(|(_, differences)| !differences.is_empty())
        .collect();

    assert!(differences.is_empty(), "{differences:?}");
}

#[test]
fn should_include_unitys_characters() {
    let built = built_recordings();

    let differences: Vec<_> = built
        .iter()
        .map(|font| {
            (
                font.name(),
                font.character_differences(),
                font.font.unmapped_characters.clone(),
            )
        })
        .filter(|(_, differences, _)| !differences.is_empty())
        .collect();

    assert!(differences.is_empty(), "{differences:?}");
}

#[test]
fn should_compute_unitys_glyph_metrics_exactly() {
    let built = built_recordings();

    let differences: Vec<_> = built
        .iter()
        .map(|font| (font.name(), font.metric_difference()))
        .filter(|(_, difference)| difference.glyphs > 0)
        .collect();

    assert!(differences.is_empty(), "{differences:?}");
}

#[test]
fn should_place_every_glyph_where_unity_does() {
    let built = built_recordings();

    let differences: Vec<_> = built
        .iter()
        .map(|font| {
            (
                font.name(),
                font.rect_differences(),
                font.font.atlases.len(),
                font.images.len(),
            )
        })
        .filter(|(_, rects, ours, theirs)| !rects.is_empty() || ours != theirs)
        .collect();

    assert!(differences.is_empty(), "{differences:?}");
}

#[test]
fn should_render_unitys_atlases_within_the_known_differences() {
    let built = built_recordings();

    let measured: Vec<(&BuiltRecording, _, _)> = built
        .iter()
        .map(|font| (font, font.pixel_difference(), font.atlas_difference()))
        .collect();

    for (font, difference, atlas) in measured {
        println!("{}", font.report());

        match KnownDifference::of(font.name(), "atlas pixels") {
            Some(known) => {
                assert!(
                    difference.mean <= known.mean
                        && difference.max <= known.max
                        && atlas.max <= known.max,
                    "{}: atlas pixels differ by mean {:.4} max {} (known: {} / {}; {})",
                    font.name(),
                    difference.mean,
                    difference.max,
                    known.mean,
                    known.max,
                    known.reason
                );
                assert!(
                    difference.max > 0,
                    "{}: atlas pixels no longer differ; drop the known difference",
                    font.name()
                );
            }
            None => assert_eq!((difference.max, atlas.max), (0, 0), "{}", font.name()),
        }
    }
}
