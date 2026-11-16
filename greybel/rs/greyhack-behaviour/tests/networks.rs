use greyhack_behaviour::{flood_reboot_threshold, map_distance, signal_strength};

#[test]
fn should_truncate_wifi_strength_from_the_unrounded_scaled_distance() {
    assert_eq!(signal_strength(0.005), 100);
}

#[test]
fn should_compute_map_distance_with_float_deltas_and_double_squares() {
    assert_eq!(map_distance([0.01, 0.05], [0.0, 0.0]), 0.050_990_194);
    assert_ne!(
        map_distance([0.01, 0.05], [0.0, 0.0]),
        (0.01_f32.powi(2) + 0.05_f32.powi(2)).sqrt()
    );
}

#[test]
fn should_reboot_at_three_times_the_net_speed_with_a_super_administrator() {
    assert_eq!(flood_reboot_threshold(6, true), 18);
}

#[test]
fn should_reboot_at_three_quarters_of_that_without_a_super_administrator() {
    assert_eq!(flood_reboot_threshold(6, false), 14);
    assert_eq!(flood_reboot_threshold(3, false), 7);
}
