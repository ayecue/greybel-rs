use super::{FloodWarning, start_text};

#[test]
fn should_warn_about_crownshield_without_naming_a_super_administrator() {
    let text = start_text("1.2.3.4", &FloodWarning::CrownShield);

    assert!(text.contains("Press Ctrl+C to stop\nWarning: DDoS traffic blocked by CrownShield."));
    assert!(!text.contains("Super administrator"));
}

#[test]
fn should_print_no_warning_for_a_plain_flood() {
    assert_eq!(
        start_text("1.2.3.4", &FloodWarning::None),
        "Flooding 1.2.3.4...\nPress Ctrl+C to stop"
    );
}
