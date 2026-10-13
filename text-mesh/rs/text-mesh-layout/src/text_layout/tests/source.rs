use super::*;

#[path = "support/source.rs"]
mod support;

#[test]
fn should_end_character_spacing_where_a_spaced_run_closes() {
    let fonts = support::font_set();
    let spaced = Style {
        character_spacing: 4.0,
        ..support::style()
    };
    let text = Text {
        runs: vec![
            Run::Text {
                text: "ab".to_owned(),
                style: spaced,
            },
            Run::Layout(LayoutChange::IndentEnd),
            Run::Text {
                text: " ".to_owned(),
                style: support::style(),
            },
        ],
    };

    let source = Source::new(&text, &fonts, &TextAssetTable::EMPTY, 16.5);

    assert_eq!(
        source.tokens,
        [
            Token::Element(0),
            Token::Element(1),
            Token::CharacterSpacingEnd,
            Token::Layout(LayoutChange::IndentEnd),
            Token::Element(2),
        ]
    );
}

#[test]
fn should_give_a_line_break_the_style_of_the_text_before_it() {
    let fonts = support::font_set();
    let large = Style {
        size: 30.0,
        ..support::style()
    };
    let text = Text {
        runs: vec![
            Run::Text {
                text: "a".to_owned(),
                style: large,
            },
            Run::LineBreak,
            Run::Text {
                text: "b".to_owned(),
                style: support::style(),
            },
        ],
    };

    let source = Source::new(&text, &fonts, &TextAssetTable::EMPTY, 16.5);

    let line_break = source.elements[1];

    assert_eq!((line_break.unicode(), line_break.style.size), (10, 30.0));
}

#[test]
fn should_skip_a_sprite_the_sprite_asset_lacks() {
    let fonts = support::font_set();
    let text = Text {
        runs: vec![Run::Sprite {
            index: 3,
            color: Color::WHITE,
            style: support::style(),
        }],
    };

    let source = Source::new(&text, &fonts, &TextAssetTable::EMPTY, 16.5);

    assert!(source.tokens.is_empty() && source.elements.is_empty());
}
