use text_mesh_test_support::fonts::{font, font_set};

#[test]
fn should_find_a_character_in_the_fallbacks_of_a_fallback() {
    let fonts = font_set(
        vec![
            font("primary", "a", 1.0, &[1]),
            font("subset", "b", 2.0, &[2]),
            font("third_level", "c", 3.0, &[]),
        ],
        &[],
    );

    let found = fonts.glyph(0, 'c');

    assert_eq!(
        found.map(|(font, glyph)| (font, glyph.advance)),
        Some((2, 3.0))
    );
}

#[test]
fn should_search_the_primary_font_before_the_global_fallbacks() {
    let fonts = font_set(
        vec![
            font("primary", "a", 1.0, &[]),
            font("tagged", "b", 2.0, &[]),
            font("global", "a", 3.0, &[]),
        ],
        &[2],
    );

    let found = fonts.glyph(1, 'a');

    assert_eq!(found.map(|(font, _)| font), Some(0));
}

#[test]
fn should_search_the_global_fallbacks_last() {
    let fonts = font_set(
        vec![
            font("primary", "a", 1.0, &[1]),
            font("subset", "b", 2.0, &[]),
            font("global", "c", 3.0, &[]),
        ],
        &[2],
    );

    let found = fonts.glyph(0, 'c');

    assert_eq!(found.map(|(font, _)| font), Some(2));
}

#[test]
fn should_replace_a_missing_character_by_the_missing_glyph() {
    let fonts = font_set(
        vec![
            font("primary", "a", 1.0, &[]),
            font("global", "?", 3.0, &[]),
        ],
        &[1],
    );

    let element = fonts.element(0, '\u{4f60}');

    assert_eq!((element.0, element.1, element.2.advance), ('?', 1, 3.0));
}

#[test]
fn should_fall_back_to_a_space_without_a_missing_glyph() {
    let fonts = font_set(vec![font("primary", " ", 1.0, &[])], &[]);

    let element = fonts.element(0, '\u{4f60}');

    assert_eq!((element.0, element.1), (' ', 0));
}
