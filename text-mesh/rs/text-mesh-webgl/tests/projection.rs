use text_mesh_webgl::*;

#[test]
fn should_map_the_frame_corners_to_the_clip_space_corners() {
    let projection = Projection {
        width: 200.0,
        height: 50.0,
        pixel_ratio: 2.0,
    };
    let m = projection.matrix();
    let clip = |x: f32, y: f32| [m[0] * x + m[4] * y + m[12], m[1] * x + m[5] * y + m[13]];

    let corners = [clip(0.0, 0.0), clip(200.0, 50.0)];

    assert_eq!(corners, [[-1.0, 1.0], [1.0, -1.0]]);
}

#[test]
fn should_measure_the_screen_in_device_pixels() {
    let projection = Projection {
        width: 1000.0,
        height: 44.0,
        pixel_ratio: 1.5,
    };

    let screen = projection.screen();

    assert_eq!(screen, [1500.0, 66.0]);
}

#[test]
fn should_give_the_sdf_scale_unity_gives_a_pixel_aligned_canvas() {
    let projection = Projection {
        width: 1000.0,
        height: 4096.0,
        pixel_ratio: 1.0,
    };
    let m = projection.matrix();
    let [width, height] = projection.screen();
    let pixel_size = [1.0 / (m[0] * width).abs(), 1.0 / (m[5] * height).abs()];
    let unity_pixel_size = 1.0 / (2.0 / 10.0 * 4096.0_f32);
    let unity_lossy_scale = 10.0 / 4096.0_f32;

    let scale = (pixel_size[0] * pixel_size[0] + pixel_size[1] * pixel_size[1])
        .sqrt()
        .recip();

    let unity = (2.0 * unity_pixel_size * unity_pixel_size).sqrt().recip() * unity_lossy_scale;

    assert!((scale - unity).abs() < 1e-5, "{scale} vs {unity}");
}
