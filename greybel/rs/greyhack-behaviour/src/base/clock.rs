//! Parses and formats game dates without using the host clock.

/// Number of world seconds per real second (`Clock.multTiempo`).
pub const TIME_SCALE: u64 = 14;
/// Number of seconds in one world day.
pub const SECONDS_PER_DAY: u64 = 86_400;
/// Number of seconds in one world hour.
pub const SECONDS_PER_HOUR: u64 = 3_600;
/// Number of seconds in one world minute.
pub const SECONDS_PER_MINUTE: u64 = 60;
/// Number of milliseconds in one real second.
pub const MILLIS_PER_SECOND: u64 = 1000;

/// World seconds that `ms` real milliseconds last, rounded up (`Task.Delay`).
#[must_use]
pub const fn world_seconds(ms: u64) -> u64 {
    (ms * TIME_SCALE).div_ceil(MILLIS_PER_SECOND)
}

/// World seconds `ticks` ticks add at `rate` ticks per real second given `carry` units left over
/// from earlier ticks; returns the seconds and the new carry. Each tick adds [`TIME_SCALE`] /
/// `rate` seconds, so `rate` ticks add exactly [`TIME_SCALE`] seconds at every rate. A carry of
/// `rate` or more (from a faster rate) is dropped.
#[must_use]
pub fn advance_ticks(carry: u32, ticks: u64, rate: u32) -> (u64, u32) {
    let rate = u128::from(rate.max(1));
    let carry = if u128::from(carry) >= rate { 0 } else { carry };
    let units = u128::from(carry) + u128::from(ticks) * u128::from(TIME_SCALE);

    (
        u64::try_from(units / rate).unwrap_or(u64::MAX),
        u32::try_from(units % rate).unwrap_or(0),
    )
}

/// World time after `ticks` ticks at `rate` ticks per real second, starting from `seconds` and
/// ignoring any carry, so it is never earlier than the time [`advance_ticks`] reaches.
#[must_use]
pub fn time_after_ticks(seconds: u64, ticks: u64, rate: u32) -> u64 {
    let span = ticks
        .saturating_mul(TIME_SCALE)
        .div_ceil(u64::from(rate.max(1)));

    seconds.saturating_add(span)
}

/// Starting world date from `Clock`'s `iniDia`, `iniMes`, `iniYear`, and `iniHora`.
pub const START_DATE: &str = "03/Jan/2000 - 06:00";

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Converts `01/Jan/2000 - 00:00` text to Unix-epoch seconds, returning 0 for other formats.
#[must_use]
pub fn parse_date(text: &str) -> u64 {
    try_parse_date(text).unwrap_or(0)
}

/// Converts `01/Jan/2000 - 00:00` text to Unix-epoch seconds, returning `None` for other formats.
#[must_use]
pub fn try_parse_date(text: &str) -> Option<u64> {
    let (date, time) = text.split_once(" - ")?;
    let mut date = date.split('/');
    let (Some(day), Some(month), Some(year)) = (date.next(), date.next(), date.next()) else {
        return None;
    };
    let (Ok(day), Ok(year)) = (day.parse::<u32>(), year.parse::<i64>()) else {
        return None;
    };
    let month = MONTHS.iter().position(|name| *name == month)? + 1;
    let mut time = time.split(':');
    let (Some(hour), Some(minute)) = (time.next(), time.next()) else {
        return None;
    };
    let (Ok(hour), Ok(minute)) = (hour.parse::<u32>(), minute.parse::<u32>()) else {
        return None;
    };

    if hour > 23 || minute > 59 || day == 0 || day > 31 {
        return None;
    }

    Some(
        (u64::try_from(
            days_from_civil(
                year,
                u32::try_from(month).expect("the month position fits u32"),
                day,
            )
            .max(0),
        )
        .expect("clamped days are nonnegative"))
            * SECONDS_PER_DAY
            + u64::from(hour * 3_600 + minute * 60),
    )
}

/// Formats Unix-epoch seconds for `current_date`.
#[must_use]
#[expect(
    clippy::cast_possible_wrap,
    reason = "The signed civil-date algorithm intentionally wraps the Unix-day count"
)]
pub fn format_date(seconds: u64) -> String {
    let days = (seconds / SECONDS_PER_DAY) as i64;
    let within_day = seconds % SECONDS_PER_DAY;
    let (year, month, day) = civil_from_days(days);

    format!(
        "{day:02}/{}/{year:04} - {:02}:{:02}",
        MONTHS[(month - 1) as usize],
        within_day / 3_600,
        (within_day % 3_600) / 60
    )
}

/// Formats Unix-epoch seconds for `file.date` (`FileIntrinsics.date`).
#[must_use]
pub fn format_file_date(seconds: u64) -> String {
    format_date(seconds).replace(" - ", " ")
}

/// Returns the calendar year, month, and day of `at`.
#[must_use]
#[expect(
    clippy::cast_possible_wrap,
    reason = "The signed civil-date algorithm intentionally wraps the Unix-day count"
)]
pub fn date_parts(at: u64) -> (i64, u32, u32) {
    civil_from_days((at / SECONDS_PER_DAY) as i64)
}

/// Adds one calendar month to `at`, clamping the day like `DateTime.AddMonths(1)`.
#[must_use]
pub fn add_month(at: u64) -> u64 {
    add_months(at, 1)
}

/// Adds `months` calendar months to `at`, clamping the day like `DateTime.AddMonths`.
#[must_use]
pub fn add_months(at: u64, months: u32) -> u64 {
    let (year, month, day) = date_parts(at);
    let month = i64::from(month - 1) + i64::from(months);
    let year = year + month.div_euclid(12);
    let month =
        u32::try_from(month.rem_euclid(12)).expect("Euclidean month remainder is nonnegative") + 1;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };

    u64::try_from(days_from_civil(year, month, day.min(days)).max(0))
        .expect("clamped days are nonnegative")
        * SECONDS_PER_DAY
        + at % SECONDS_PER_DAY
}

/// Returns the one-based day of `at` within its calendar year (`DateTime.DayOfYear`).
#[must_use]
#[expect(
    clippy::cast_possible_wrap,
    reason = "The signed civil-date algorithm intentionally wraps the Unix-day count"
)]
pub fn day_of_year(at: u64) -> u32 {
    let days = (at / SECONDS_PER_DAY) as i64;
    let (year, _, _) = civil_from_days(days);

    u32::try_from(days - days_from_civil(year, 1, 1) + 1).expect("the day of year is positive")
}

/// Formats Unix-epoch seconds as the auto-patch date shown by `is_patched(1)`.
#[must_use]
pub fn format_auto_patch_date(seconds: u64) -> String {
    let (year, month, day) = date_parts(seconds);

    format!("{day:02}/{month:02}/{year:04}")
}

/// Checks whether `at` is a Saturday or Sunday.
#[must_use]
pub const fn is_weekend(at: u64) -> bool {
    // Unix day zero was Thursday, three days after Monday.
    matches!((at / SECONDS_PER_DAY + 3) % 7, 5 | 6)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = (i64::from(month) + 9) % 12;
    let day_of_year = (153 * month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month + 2) / 5 + 1;
    let month = month + if month < 10 { 3 } else { -9 };

    (
        year + i64::from(month <= 2),
        u32::try_from(month).expect("civil month is positive"),
        u32::try_from(day).expect("civil day is positive"),
    )
}
