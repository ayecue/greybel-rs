use greyhack_text_mesh_assets::{TERMINAL_TEXT, TERMINAL_THEME};
use text_mesh_webgl::Vertex;

use crate::support::output;
use crate::support::service::TerminalHarness;
use crate::support::service::page::{Material, Scroll, Textures};

#[test]
fn should_give_the_page_every_atlas_of_the_fonts_and_the_sprite_sheet() {
    let harness = TerminalHarness::new();
    let atlases: usize = TERMINAL_TEXT
        .font_set
        .fonts
        .iter()
        .map(|font| font.atlas.map_or(0, |atlas| usize::from(atlas.count)))
        .sum();

    let pending = harness.page.send(&Textures {}).expect("sent");
    let textures = harness.wait(pending).expect("textures");

    let sheet = TERMINAL_TEXT.sprite_sheet;

    assert_eq!(
        (
            textures.atlases.len(),
            textures.sprite_sheet.width,
            textures.sprite_sheet.rgba.len()
        ),
        (atlases, u32::from(sheet.width), sheet.rgba.len())
    );
    for atlas in &textures.atlases {
        assert_eq!(atlas.alpha.len(), (atlas.width * atlas.height) as usize);
    }
}

#[test]
fn should_request_the_page_to_draw_again_when_the_output_changes() {
    let harness = TerminalHarness::new();

    harness.print("hello");

    assert_eq!(harness.redraws.get(), 1);
}

#[test]
fn should_draw_the_output_in_device_pixels_with_the_games_sampling() {
    let harness = TerminalHarness::new();

    harness.print("hello");

    let scene = harness.draw();

    let background = TERMINAL_THEME.background;

    assert_eq!(
        (
            scene.frame.width,
            scene.frame.height,
            scene.frame.projection.len(),
            scene.frame.samples.len(),
            scene.frame.anisotropy,
            [
                scene.frame.background.r,
                scene.frame.background.g,
                scene.frame.background.b
            ],
        ),
        (
            2000,
            600,
            16,
            2,
            16,
            [background.r, background.g, background.b]
        )
    );
    assert!(
        scene
            .meshes
            .iter()
            .any(|mesh| matches!(mesh.material, Material::Font { .. }))
    );
    for mesh in &scene.meshes {
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len() % (4 * Vertex::STRIDE), 0);
    }
}

#[test]
fn should_draw_nothing_before_the_program_prints() {
    let harness = TerminalHarness::new();

    let scene = harness.draw();

    assert!(scene.meshes.is_empty());
}

#[test]
fn should_draw_what_it_was_scrolled_to() {
    let harness = TerminalHarness::new();

    harness.print(&output(40));

    let following = harness.draw();

    harness.page.send(&Scroll { delta: -200.0 }).expect("sent");

    let scrolled = harness.draw();

    assert_ne!(scrolled.meshes, following.meshes);
}
