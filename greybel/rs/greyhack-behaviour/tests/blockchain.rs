use greyhack_behaviour::hash_power;

#[test]
fn should_add_the_power_of_cpus_rams_and_gpus() {
    let power = hash_power([(2.0, 2)], [(1_000, 2)], [10.0]);

    assert!((power - (2.0 * 2.0 * 0.15 + 1_000.0 * 0.0005 * 2.0 + 10.0)).abs() < 1e-6);
    assert_eq!(hash_power([], [], []), 0.0);
}
