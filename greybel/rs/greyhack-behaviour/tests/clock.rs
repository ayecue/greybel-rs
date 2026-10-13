use greyhack_behaviour::{SECONDS_PER_DAY, add_month, format_date, is_weekend, parse_date};

#[test]
fn should_round_trip_game_dates() {
    let dates = [
        "01/Jan/2000 - 00:00",
        "29/Feb/2024 - 23:59",
        "31/Oct/2026 - 12:34",
    ];

    let formatted: Vec<String> = dates
        .iter()
        .map(|date| format_date(parse_date(date)))
        .collect();

    assert_eq!(formatted, dates);
}

#[test]
fn should_advance_over_a_day_boundary() {
    let start = parse_date("31/Oct/2026 - 23:59");

    let next = format_date(start + 60);

    assert_eq!(next, "01/Nov/2026 - 00:00");
}

#[test]
fn should_default_an_unreadable_date_to_the_epoch() {
    let text = "not a date";

    let seconds = parse_date(text);

    assert_eq!(seconds, 0);
}

#[test]
fn should_add_calendar_months_like_the_game() {
    let months = [
        "31/Jan/2000 - 06:00",
        "31/Dec/2000 - 06:00",
        "15/Mar/2001 - 23:59",
    ]
    .map(|date| format_date(add_month(parse_date(date))));

    assert_eq!(
        months,
        [
            "29/Feb/2000 - 06:00",
            "31/Jan/2001 - 06:00",
            "15/Apr/2001 - 23:59"
        ]
    );
}

#[test]
fn should_keep_the_seconds_of_the_day_a_month_later() {
    let at = parse_date("10/May/2001 - 12:00") + 42;

    assert_eq!(add_month(at) % SECONDS_PER_DAY, at % SECONDS_PER_DAY);
}

#[test]
fn should_tell_weekends_from_weekdays() {
    let days = [
        "06/Jan/2001 - 12:00",
        "07/Jan/2001 - 12:00",
        "08/Jan/2001 - 12:00",
    ]
    .map(|date| is_weekend(parse_date(date)));

    assert_eq!(days, [true, true, false]);
}
