use greyhack_behaviour::SmartApplianceSettings;

#[test]
fn should_break_until_the_recovery_time_and_lock_the_override() {
    let mut settings = SmartApplianceSettings::new("fridge", 600, 20);

    assert!(settings.has_required_values());
    assert!(!settings.is_broken(100));

    settings.override_enabled = true;
    settings.break_until(200);

    assert!(settings.is_broken(199));
    assert!(!settings.is_broken(200));
    assert!(settings.alarm);
    assert!(!settings.override_enabled);
    assert!(!settings.has_required_values());
}
