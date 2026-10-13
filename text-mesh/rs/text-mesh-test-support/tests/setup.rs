//! The exported setup gives every font `setup.json` names, with its atlases.

use support::setup::names;

use crate::support;

#[test]
fn should_export_every_font_of_the_setup() {
    let setup = text_mesh_test_support::setup();

    let names = names(&setup.fonts);

    assert_eq!(
        names,
        [
            "FiraMono-Regular SDF",
            "Sarabun-Regular SDF",
            "LiberationSans SDF"
        ]
    );
}

#[test]
fn should_export_every_fonts_atlases() {
    let setup = text_mesh_test_support::setup();

    let missing: Vec<String> = setup
        .fonts
        .fonts
        .iter()
        .flat_map(|font| {
            let count = font.atlas.map_or(0, |atlas| atlas.count);

            (0..count).map(move |index| format!("{}.{index}.png", font.name))
        })
        .filter(|file| {
            !text_mesh_test_support::folder()
                .join("atlases")
                .join(file)
                .exists()
        })
        .collect();

    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn should_make_the_terminals_font_primary() {
    let setup = text_mesh_test_support::setup();

    let primary = &setup.fonts.fonts[setup.fonts.primary].name;

    assert_eq!(primary, "FiraMono-Regular SDF");
}
