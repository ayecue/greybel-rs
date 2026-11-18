//! Verifies that bundle contents match their fingerprint and that setup-referenced assets are consistent with the
//! setup.

use asset_bundle::{Bundle, Fingerprint};
use greyhack_text_mesh_assets::{AtlasFile, FontFile, TERMINAL_TEXT};
use text_mesh_setup::Setup;

#[test]
fn should_hold_the_files_its_fingerprint_lists() {
    let committed = include_str!("../fingerprint.json");
    let fingerprint = Fingerprint::read(committed).expect("fingerprint.json is a fingerprint");
    let bundle =
        Bundle::read(include_bytes!("../assets.bundle")).expect("assets.bundle is a bundle");

    let actual = bundle.fingerprint(&fingerprint.game);

    assert_eq!(
        actual.to_json(),
        committed,
        "run `make assets` in greybel/dotnet"
    );
}

#[test]
fn should_hold_every_atlas_of_every_font_at_its_size() {
    let fonts = &TERMINAL_TEXT.font_set.fonts;

    let mismatched: Vec<String> = fonts
        .iter()
        .flat_map(|font| {
            let atlas = font.atlas.expect("the game's fonts have atlases");

            (0..atlas.count).filter_map(move |index| {
                let size = AtlasFile::find(&font.name, index).map(|file| file.alpha.len());
                let expected = usize::from(atlas.width) * usize::from(atlas.height);

                (size != Some(expected)).then(|| format!("{} atlas {index}: {size:?}", font.name))
            })
        })
        .collect();

    assert!(mismatched.is_empty(), "{mismatched:?}");
}

#[test]
fn should_hold_the_sprite_sheet_at_its_texture_size() {
    let sheet = TERMINAL_TEXT.sprite_sheet;

    let size = sheet.rgba.len();

    assert_eq!(
        size,
        usize::from(sheet.width) * usize::from(sheet.height) * 4
    );
}

#[test]
fn should_place_every_glyph_inside_its_atlas() {
    let fonts = &TERMINAL_TEXT.font_set.fonts;

    let outside: Vec<String> = fonts
        .iter()
        .flat_map(|font| {
            let atlas = font.atlas.expect("the game's fonts have atlases");

            font.glyphs.iter().filter_map(move |glyph| {
                let rect = glyph.rect;
                let inside = rect.atlas < atlas.count
                    && rect.x + rect.width <= atlas.width
                    && rect.y + rect.height <= atlas.height;

                (!inside).then(|| format!("{} {:?}: {rect:?}", font.name, glyph.character))
            })
        })
        .collect();

    assert!(outside.is_empty(), "{outside:?}");
}

#[test]
fn should_keep_the_glyph_and_kerning_tables_sorted() {
    let fonts = &TERMINAL_TEXT.font_set.fonts;

    let unsorted: Vec<&str> = fonts
        .iter()
        .filter(|font| !font.is_sorted())
        .map(|font| font.name.as_ref())
        .collect();

    assert!(unsorted.is_empty(), "{unsorted:?}");
}

#[test]
fn should_have_a_font_file_for_every_font_asset_made_from_one() {
    let bundle =
        Bundle::read(include_bytes!("../assets.bundle")).expect("assets.bundle is a bundle");
    let setup = Setup::read(
        bundle
            .text("text-mesh.json")
            .expect("the bundle has text-mesh.json"),
    )
    .expect("text-mesh.json is the game's setup");

    let sources = setup.font_sources();
    let missing: Vec<&str> = sources
        .iter()
        .map(|&(name, _)| name)
        .filter(|name| FontFile::find(name).is_none())
        .collect();

    assert!(!sources.is_empty());
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn should_read_the_games_terminal_settings() {
    let terminal = &*TERMINAL_TEXT;

    let read = (
        terminal.settings.font_size,
        terminal.settings.line_spacing,
        terminal.frame.padding,
        terminal.frame.item_min_height,
    );

    assert_eq!(read, (16.5, 4.5, 10.0, 24.0));
}
