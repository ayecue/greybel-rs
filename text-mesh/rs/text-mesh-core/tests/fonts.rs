//! Font and material tags, which both push onto one stack of material references.

use text_mesh_core::{TextAssetTable, TextComponent};

use crate::support::{ASSETS, Color, component_in, style_of};

#[test]
fn should_set_the_font_of_the_following_text() {
    let text = "<font=\"LiberationSans SDF\">a</font>b";
    let shown = [style_of(text, "a").font, style_of(text, "b").font];

    assert_eq!(shown, [Some(0), None]);
}

#[test]
fn should_find_a_font_by_its_name_in_any_case() {
    let text = "<font=\"sourcecodepro-regular sdf\">a";
    let shown = style_of(text, "a").font;

    assert_eq!(shown, Some(1));
}

#[test]
fn should_go_back_to_the_component_font_with_default() {
    let text = "<font=\"LiberationSans SDF\"><font=default>a";
    let shown = style_of(text, "a").font;

    assert_eq!(shown, None);
}

#[test]
fn should_set_a_material_with_its_font() {
    let text = "<font=\"LiberationSans SDF\" material=\"LiberationSans SDF - Outline\">a";
    let style = style_of(text, "a");

    assert_eq!((style.font, style.material), (Some(0), Some(0)));
}

#[test]
fn should_keep_the_font_when_closing_a_material() {
    let text = "<font=\"LiberationSans SDF\"><material=\"SourceCodePro-Regular SDF\">a</material>b";
    let shown =
        [style_of(text, "a"), style_of(text, "b")].map(|style| (style.font, style.material));

    assert_eq!(shown, [(Some(0), Some(1)), (Some(0), None)]);
}

#[test]
fn should_close_a_material_with_a_closing_font_as_the_game_does() {
    let text = "<font=\"LiberationSans SDF\"><material=\"SourceCodePro-Regular SDF\">a</font>b";
    let shown = style_of(text, "b");

    assert_eq!((shown.font, shown.material), (Some(0), None));
}

#[test]
fn should_show_fonts_and_materials_as_text_without_assets() {
    let assets = TextAssetTable::EMPTY;
    let component = TextComponent::new(Color::WHITE, 20.0, &assets);
    let text = "<font=\"LiberationSans SDF\">a<material=\"SourceCodePro-Regular SDF\">b";
    let shown = component.parse(text).plain();

    assert_eq!(shown, text);
}

#[test]
fn should_take_its_fonts_from_the_assets_it_is_given() {
    let mut assets = ASSETS.clone();

    assets.fonts.to_mut().push("Custom SDF".into());

    let component = component_in(Color::WHITE).with_assets(&assets);
    let shown = component.parse("<font=\"Custom SDF\">a").runs.len();

    assert_eq!(shown, 1);
}
