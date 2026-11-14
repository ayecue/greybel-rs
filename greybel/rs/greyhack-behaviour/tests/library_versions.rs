use greyhack_behaviour::{
    MAX_SAVED_LIBRARIES, MAX_WEAKNESS_USES, TIME_SCALE, count_use, is_worn_out, is_zone_exhausted,
    library_check_seconds, next_version, replacement_zone_count, saved_versions_to_prune,
};

#[test]
fn should_count_each_part_of_a_version_up_to_nine() {
    assert_eq!(next_version("1.0.0"), "1.0.1");
    assert_eq!(next_version("1.0.9"), "1.1.0");
    assert_eq!(next_version("1.9.9"), "2.0.0");
    assert_eq!(next_version("2.3.4"), "2.3.5");
}

#[test]
fn should_check_for_versions_every_fifteen_real_minutes() {
    assert_eq!(library_check_seconds(), 15 * 60 * TIME_SCALE);
}

#[test]
fn should_count_uses_up_to_one_past_the_limit() {
    assert_eq!(count_use(0), 1);
    assert_eq!(count_use(MAX_WEAKNESS_USES - 1), MAX_WEAKNESS_USES);
    assert_eq!(count_use(MAX_WEAKNESS_USES), MAX_WEAKNESS_USES + 1);
    assert_eq!(count_use(MAX_WEAKNESS_USES + 1), MAX_WEAKNESS_USES + 1);
}

#[test]
fn should_wear_a_weakness_out_at_the_limit() {
    assert!(!is_worn_out(MAX_WEAKNESS_USES - 1));
    assert!(is_worn_out(MAX_WEAKNESS_USES));
}

#[test]
fn should_replace_a_zone_patched_three_times() {
    assert!(!is_zone_exhausted(2));
    assert!(is_zone_exhausted(3));
}

#[test]
fn should_prune_the_oldest_saved_versions_beyond_a_hundred() {
    assert_eq!(saved_versions_to_prune(MAX_SAVED_LIBRARIES), 0);
    assert_eq!(saved_versions_to_prune(MAX_SAVED_LIBRARIES + 3), 3);
    assert_eq!(saved_versions_to_prune(0), 0);
}

#[test]
fn should_replace_a_zone_by_one_unless_the_draw_drops_it() {
    let mut draws = [0, 9, 2].into_iter();

    assert_eq!(replacement_zone_count(3, 4, || draws.next().unwrap()), 3);

    let mut draws = [3, 5, 4].into_iter();

    assert_eq!(replacement_zone_count(3, 4, || draws.next().unwrap()), 0);
}

#[test]
fn should_replace_the_last_zone_even_when_the_draw_drops_it() {
    let mut draws = [4].into_iter();

    assert_eq!(replacement_zone_count(1, 1, || draws.next().unwrap()), 1);
}

#[test]
fn should_make_an_extra_zone_for_a_draw_of_six_or_seven() {
    let mut draws = [6, 7].into_iter();

    assert_eq!(replacement_zone_count(2, 5, || draws.next().unwrap()), 4);
}

#[test]
fn should_stop_making_extra_zones_past_the_bound() {
    let mut draws = std::iter::repeat(6);

    // Each replaced zone makes two while at most five were made, then one.
    assert_eq!(replacement_zone_count(4, 8, || draws.next().unwrap()), 7);
}
