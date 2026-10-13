use greyhack_behaviour::flood_reboot_threshold;

#[test]
fn should_reboot_at_three_times_the_net_speed_with_a_super_administrator() {
    assert_eq!(flood_reboot_threshold(6, true), 18);
}

#[test]
fn should_reboot_at_three_quarters_of_that_without_a_super_administrator() {
    assert_eq!(flood_reboot_threshold(6, false), 14);
    assert_eq!(flood_reboot_threshold(3, false), 7);
}
