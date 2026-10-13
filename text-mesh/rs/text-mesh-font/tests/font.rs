use text_mesh_font::{GlyphAdjustment, GlyphRect, *};

#[test]
fn should_synthesize_metrics_for_control_and_format_characters_missing_from_a_font() {
    let font = FontAsset::default();

    let glyphs = ['\t', '\n', '\u{200b}', 'a'].map(|character| font.glyph(character));

    assert_eq!(
        glyphs,
        [
            Some(GlyphMetrics::default()),
            Some(GlyphMetrics::default()),
            Some(GlyphMetrics::default()),
            None
        ]
    );
}

#[test]
fn should_take_an_unset_cap_line_from_the_capital_x() {
    let font = FontAsset::default().with_glyphs([Glyph {
        character: 'X',
        metrics: GlyphMetrics {
            bearing_y: 27.5,
            ..GlyphMetrics::default()
        },
        rect: GlyphRect::default(),
    }]);

    let cap_line = font.cap_line();

    assert_eq!(cap_line, 27.5);
}

#[test]
fn should_find_glyphs_given_in_any_order() {
    let font = FontAsset::default().with_glyphs("cab".chars().map(|character| Glyph {
        character,
        metrics: GlyphMetrics {
            advance: f32::from(u8::try_from(character).unwrap_or_default()),
            ..GlyphMetrics::default()
        },
        rect: GlyphRect::default(),
    }));

    let advances =
        ['a', 'b', 'c'].map(|character| font.glyph(character).map(|glyph| glyph.advance));

    assert_eq!(advances, [Some(97.0), Some(98.0), Some(99.0)]);
}

#[test]
fn should_find_a_kerning_pair_by_both_characters() {
    let pair = |first, second, x_advance| Kerning {
        first,
        second,
        pair: KerningPair {
            first: GlyphAdjustment {
                x_advance,
                ..GlyphAdjustment::default()
            },
            ..KerningPair::default()
        },
    };
    let font = FontAsset::default().with_kerning([pair('V', 'A', 2.0), pair('A', 'V', -1.0)]);

    let found = (font.kerning('A', 'V'), font.kerning('V', 'V'));

    assert_eq!(
        (found.0.map(|pair| pair.first.x_advance), found.1),
        (Some(-1.0), None)
    );
}
