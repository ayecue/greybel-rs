use greyhack_behaviour::{NpcSchedule, Rng, SECONDS_PER_DAY, day_of_year, pending_wake_up};

use crate::support::npc_schedule::schedule;

#[test]
fn should_use_half_open_weekday_work_blocks_and_rest_on_weekends() {
    // 03/Jan/2000 00:00 is 946,857,600 Unix seconds; the generated worlds begin at 06:00.
    let monday = 946_857_600;
    let schedule = schedule(true, 0, 0);

    assert!(!schedule.is_working(monday + 7 * 3_600 + 29 * 60));
    assert!(schedule.is_working(monday + 7 * 3_600 + 30 * 60));
    assert!(schedule.is_working(monday + 11 * 3_600 + 29 * 60));
    assert!(!schedule.is_working(monday + 11 * 3_600 + 30 * 60));
    assert!(schedule.is_working(monday + 12 * 3_600));
    assert!(!schedule.is_working(monday + 16 * 3_600));
    assert!(!schedule.is_working(monday + 5 * SECONDS_PER_DAY + 12 * 3_600));
    assert!(!schedule.is_working(monday + 6 * SECONDS_PER_DAY + 12 * 3_600));
}

#[test]
fn should_carry_night_work_over_midnight_and_evaluate_at_the_next_boundary() {
    let monday = 946_857_600;
    let schedule = schedule(false, 0, 0);

    assert!(schedule.is_working(monday + 20 * 3_600 + 30 * 60));
    assert!(schedule.is_working(monday + SECONDS_PER_DAY + 15 * 60));
    assert_eq!(
        schedule.next_boundary(monday + 23 * 3_600),
        monday + SECONDS_PER_DAY + 30 * 60
    );
    assert_eq!(
        schedule.next_boundary(monday + 5 * SECONDS_PER_DAY + 23 * 3_600),
        monday + 7 * SECONDS_PER_DAY + 20 * 3_600 + 30 * 60
    );
}

#[test]
fn should_draw_the_games_quarter_hour_offsets() {
    let mut rng = Rng::new(7);
    let day = NpcSchedule::new(&mut rng, true);
    let night = NpcSchedule::new(&mut rng, false);

    for schedule in [&day, &night] {
        assert_eq!((schedule.work1_start() - 1_800) % 900, 0);
    }

    assert!(matches!(day.work1_start() - 1_800, 25_200..=27_900));
    assert!(matches!(night.work1_start() - 1_800, 72_000..=74_700));
}

#[test]
fn should_read_back_the_schedule_it_stores() {
    let schedule = NpcSchedule::new(&mut Rng::new(3), false);
    let stored = NpcSchedule::from_schedule(
        schedule.work1_start(),
        schedule.work2_start(),
        schedule.walk_to_appliance(),
    );

    assert_eq!(stored, schedule);
}

#[test]
fn should_be_at_work_through_the_break_between_work_blocks() {
    let monday = 946_857_600;
    let day = schedule(true, 0, 0);
    let night = schedule(false, 0, 0);

    assert!(!day.is_at_work(monday + 7 * 3_600 + 29 * 60));
    assert!(day.is_at_work(monday + 7 * 3_600 + 30 * 60));
    assert!(!day.is_working(monday + 11 * 3_600 + 45 * 60));
    assert!(day.is_at_work(monday + 11 * 3_600 + 45 * 60));
    assert!(day.is_at_work(monday + 15 * 3_600 + 59 * 60));
    assert!(!day.is_at_work(monday + 16 * 3_600));
    assert!(!day.is_at_work(monday + 5 * SECONDS_PER_DAY + 12 * 3_600));
    assert!(night.is_at_work(monday + SECONDS_PER_DAY + 30 * 60));
    assert!(!night.is_at_work(monday + 12 * 3_600));
}

#[test]
fn should_wake_up_on_the_alarm_day_when_it_is_still_ahead() {
    let monday = 946_857_600;
    let today = day_of_year(monday);

    assert_eq!(
        pending_wake_up(Some(3_600), today, monday),
        Some(monday + 3_600)
    );
    assert_eq!(
        pending_wake_up(Some(3_600), today + 1, monday),
        Some(monday + SECONDS_PER_DAY + 3_600)
    );
    assert_eq!(pending_wake_up(Some(3_600), today + 2, monday), None);
    assert_eq!(pending_wake_up(Some(3_600), today, monday + 3_600), None);
    assert_eq!(pending_wake_up(None, today, monday), None);
}

#[test]
fn should_look_at_the_npc_at_the_earliest_of_boundary_forced_login_and_wake_up() {
    let monday = 946_857_600;
    let today = day_of_year(monday);
    let schedule = schedule(true, 0, 0);
    let boundary = schedule.next_boundary(monday);

    assert_eq!(schedule.next_switch(None, None, today, monday), boundary);
    assert_eq!(
        schedule.next_switch(Some(monday + 60), None, today, monday),
        monday + 60
    );
    assert_eq!(
        schedule.next_switch(Some(monday), None, today, monday),
        boundary
    );
    assert_eq!(
        schedule.next_switch(None, Some(120), today, monday),
        monday + 120
    );
}
