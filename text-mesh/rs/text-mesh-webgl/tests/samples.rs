use text_mesh_webgl::*;

#[test]
fn should_sample_twice_on_the_diagonal_from_bottom_left_to_top_right() {
    let samples = 2;

    let offsets = SamplePattern::offsets(samples);

    assert_eq!(offsets, [[0.25, -0.25], [-0.25, 0.25]]);
}

#[test]
fn should_keep_every_pattern_centered_on_the_pixel() {
    let counts = [1, 2, 4, 8];

    let centres: Vec<[f32; 2]> = counts
        .iter()
        .map(|&count| {
            let offsets = SamplePattern::offsets(count);
            let sum = offsets.iter().fold([0.0, 0.0], |sum, offset| {
                [sum[0] + offset[0], sum[1] + offset[1]]
            });

            [sum[0] / offsets.len() as f32, sum[1] / offsets.len() as f32]
        })
        .collect();

    assert_eq!(centres, [[0.0, 0.0]; 4]);
}
