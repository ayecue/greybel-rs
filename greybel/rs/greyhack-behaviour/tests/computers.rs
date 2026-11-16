use greyhack_behaviour::process_memory_percent;

#[test]
fn should_format_process_memory_from_the_unrounded_float_expression() {
    assert_eq!(process_memory_percent(0.01, 20), 0.049_999_998_882_412_91);
    assert_ne!(
        process_memory_percent(0.01, 20),
        f64::from(0.01_f32 * 100.0 / 20.0)
    );
}
