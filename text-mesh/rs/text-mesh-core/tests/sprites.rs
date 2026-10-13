//! Sprite tags, using the default sprite asset.

use text_mesh_core::Color;

use crate::support::plain;
use crate::support::sprites::sprites;

#[test]
fn should_show_a_sprite_by_its_index() {
    let text = "<sprite=1><sprite index=2>";
    let shown = sprites(text);

    assert_eq!(shown, vec![(1, Color::WHITE), (2, Color::WHITE)]);
}

#[test]
fn should_show_a_sprite_the_asset_lacks_as_text() {
    let text = "<sprite=3><sprite name=\"x\"><sprite name=\"Caret\"><sprite><sprite=0 foo=1>";
    let shown = plain(text);

    assert_eq!(shown, text);
}

#[test]
fn should_show_the_caret_as_the_terminal_writes_it() {
    let text = "<sprite index=0 color=#00F0DD>";
    let shown = sprites(text);

    assert_eq!(shown, vec![(0, Color::new(0, 240, 221, 255))]);
}

#[test]
fn should_read_a_quoted_index() {
    let text = "<sprite index=\"2\">";
    let shown = sprites(text);

    assert_eq!(shown, vec![(2, Color::WHITE)]);
}

#[test]
fn should_show_sprites_as_text_without_a_sprite_asset() {
    let assets = text_mesh_core::TextAssetTable::EMPTY;
    let component = text_mesh_core::TextComponent::new(Color::WHITE, 20.0, &assets);
    let shown = component.parse("<sprite=0>").plain();

    assert_eq!(shown, "<sprite=0>");
}

#[test]
fn should_find_the_first_sprite_of_a_name() {
    let text = "<sprite name=\"caret\"><sprite name=caret>";
    let shown = sprites(text);

    assert_eq!(shown, vec![(0, Color::WHITE), (0, Color::WHITE)]);
}

#[test]
fn should_color_a_sprite() {
    let text = "<sprite=0 color=#ff000080>";
    let shown = sprites(text);

    assert_eq!(shown, vec![(0, Color::new(255, 0, 0, 128))]);
}

#[test]
fn should_tint_a_sprite_with_the_text_color() {
    let text = "<color=#808080><sprite=0 tint=1>";
    let shown = sprites(text);

    assert_eq!(shown, vec![(0, Color::new(128, 128, 128, 255))]);
}
