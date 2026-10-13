use greyhack_behaviour::NpcSchedule;

/// The schedule of a worker whose day (or night) shift starts at `start_minutes` seconds after 07:00 (or 20:00) and
/// whose break lasts `break_minutes` seconds longer than the shortest.
pub fn schedule(day_time: bool, start_minutes: u64, break_minutes: u64) -> NpcSchedule {
    let start = if day_time { 7 * 3_600 } else { 20 * 3_600 } + start_minutes;
    let second_start = start + 5 * 3_600 + break_minutes;

    NpcSchedule::from_schedule(start + 1_800, second_start, (start + 16_200 + 300) % 86_400)
}
