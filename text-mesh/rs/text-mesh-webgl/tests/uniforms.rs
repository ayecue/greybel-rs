use text_mesh_webgl::*;

use crate::support::uniforms::{font_set, material};

#[test]
fn should_derive_scale_ratio_a_like_update_shader_ratios() {
    let material = material(6.0);

    let uniforms = SdfUniforms::from_material(&material);

    assert!(
        (uniforms.scale_ratio_a - 5.0 / 6.0).abs() < 1e-6,
        "{uniforms:?}"
    );
}

#[test]
fn should_take_the_gradient_scale_of_the_fallback_font() {
    let fonts = font_set();

    let uniforms = SdfUniforms::of(
        &fonts,
        MeshMaterial::Fallback {
            source: 0,
            font: 1,
            atlas: 0,
        },
    );

    assert_eq!(uniforms.map(|uniforms| uniforms.gradient_scale), Some(10.0));
}

#[test]
fn should_have_no_distance_field_uniforms_for_sprites() {
    let fonts = font_set();

    let uniforms = SdfUniforms::of(&fonts, MeshMaterial::Sprites);

    assert_eq!(uniforms, None);
}
