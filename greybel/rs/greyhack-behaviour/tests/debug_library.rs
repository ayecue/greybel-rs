use greyhack_behaviour::{
    auto_patch_time, error_lines_match, fixed_payload_user, format_date, is_patch_applied,
    parse_date,
};

#[test]
fn should_schedule_the_automatic_patch_from_the_first_of_the_month() {
    let now = parse_date("17/Mar/2001 - 13:45");

    assert_eq!(format_date(auto_patch_time(now, 0)), "01/Jun/2001 - 00:00");
    assert_eq!(format_date(auto_patch_time(now, 2)), "01/Oct/2001 - 00:00");
    assert_eq!(format_date(auto_patch_time(now, 9)), "01/Oct/2001 - 00:00");
}

#[test]
fn should_apply_a_patch_for_its_seed_or_inside_the_automatic_window() {
    assert!(is_patch_applied(5, Some(5), None, 100));
    assert!(!is_patch_applied(5, Some(4), None, 100));
    assert!(is_patch_applied(5, Some(4), Some(101), 100));
    assert!(!is_patch_applied(5, None, Some(100), 100));
}

#[test]
fn should_compare_error_lines_without_order_or_repeats() {
    assert!(error_lines_match(&[3, 1, 3], &[1, 3]));
    assert!(!error_lines_match(&[1], &[1, 3]));
}

#[test]
fn should_fix_the_payload_user_only_for_root_and_guest() {
    assert_eq!(fixed_payload_user("root"), Some("root"));
    assert_eq!(fixed_payload_user("guest"), Some("guest"));
    assert_eq!(fixed_payload_user("user"), None);
}
