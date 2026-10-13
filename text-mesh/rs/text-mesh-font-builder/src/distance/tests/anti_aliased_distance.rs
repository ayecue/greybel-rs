use super::AntiAliasedDistance;

#[test]
fn should_ramp_linearly_across_a_straight_edge() {
    let (width, height) = (12, 5);
    let coverage: Vec<u8> = (0..width * height)
        .map(|index| match index % width {
            0..=5 => 255,
            6 => 128,
            _ => 0,
        })
        .collect();

    let alpha = AntiAliasedDistance::new(width, height, 4).alpha(&coverage);

    assert_eq!(
        &alpha[2 * width + 4..2 * width + 10],
        &[191, 159, 128, 96, 64, 32]
    );
}

#[test]
fn should_render_an_empty_image_transparent() {
    let coverage = vec![0; 16];

    let alpha = AntiAliasedDistance::new(4, 4, 3).alpha(&coverage);

    assert_eq!(alpha, vec![0; 16]);
}
