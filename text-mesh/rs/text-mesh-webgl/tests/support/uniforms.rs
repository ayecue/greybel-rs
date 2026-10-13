use std::borrow::Cow;

use text_mesh_font::{FontAsset, FontSet, SdfMaterial};

pub fn material(gradient_scale: f32) -> SdfMaterial {
    SdfMaterial {
        gradient_scale,
        weight_bold: 0.75,
        ..SdfMaterial::default()
    }
}

pub fn font_set() -> FontSet {
    let font = |name: &'static str, gradient_scale: f32| FontAsset {
        name: Cow::Borrowed(name),
        material: Some(material(gradient_scale)),
        ..FontAsset::default()
    };

    text_mesh_test_support::fonts::font_set(
        vec![font("Primary SDF", 6.0), font("Fallback SDF", 10.0)],
        &[],
    )
}
