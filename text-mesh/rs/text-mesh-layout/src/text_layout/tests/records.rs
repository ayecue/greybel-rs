use super::*;

#[test]
fn should_rotate_a_point_a_quarter_turn_counterclockwise() {
    let transform = EffectTransform::new(90.0, 1.0).expect("a rotation");

    let rotated = transform.apply(Point::new(1.0, 0.0));

    assert!(
        rotated.x.abs() < 1e-6 && (rotated.y - 1.0).abs() < 1e-6,
        "{rotated:?}"
    );
}

#[test]
fn should_scale_only_horizontally() {
    let transform = EffectTransform::new(0.0, 2.0).expect("a scale");

    let scaled = transform.apply(Point::new(1.5, 3.0));

    assert_eq!(
        (scaled, transform.lossy_scale_x),
        (Point::new(3.0, 3.0), 2.0)
    );
}

#[test]
fn should_leave_unrotated_unscaled_text_untransformed() {
    let (rotation, scale) = (0.0, 1.0);

    let transform = EffectTransform::new(rotation, scale);

    assert_eq!(transform, None);
}
