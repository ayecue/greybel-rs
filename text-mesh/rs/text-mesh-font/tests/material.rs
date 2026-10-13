use text_mesh_font::*;

#[test]
fn should_compute_the_scale_ratio_of_an_undilated_material() {
    let material = SdfMaterial {
        gradient_scale: 6.0,
        weight_bold: 0.75,
        ..SdfMaterial::default()
    };

    let ratio = material.scale_ratio_a();

    assert!((ratio - 5.0 / 6.0).abs() < 1e-6, "{ratio}");
}

#[test]
fn should_pad_an_undilated_material_by_the_base_padding() {
    let material = SdfMaterial {
        gradient_scale: 10.0,
        ..SdfMaterial::default()
    };

    let padding = material.padding();

    assert_eq!(padding, 1.25);
}

#[test]
fn should_scale_the_outline_padding_by_the_scale_ratio() {
    let material = SdfMaterial {
        gradient_scale: 4.0,
        outline_width: 2.0,
        ..SdfMaterial::default()
    };

    let padding = material.padding();

    assert_eq!(padding, 2.0 * (3.0 / 8.0) * 4.0 + 1.25);
}

#[test]
fn should_take_the_atlas_properties_of_a_fallback_material() {
    let source = SdfMaterial {
        gradient_scale: 6.0,
        face_dilate: 0.25,
        ..SdfMaterial::default()
    };
    let target = SdfMaterial {
        gradient_scale: 10.0,
        weight_bold: 0.5,
        ..SdfMaterial::default()
    };

    let fallback = source.with_atlas_of(&target);

    assert_eq!(
        fallback,
        SdfMaterial {
            gradient_scale: 10.0,
            face_dilate: 0.25,
            weight_bold: 0.5,
            ..SdfMaterial::default()
        }
    );
}
