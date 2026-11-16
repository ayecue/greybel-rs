//! Wi-Fi range, signal strength, and `aircrack` capture requirements.

use crate::Rng;

/// Maximum Wi-Fi distance in map units.
pub const WIFI_RANGE: f32 = 0.5;

/// Converts router `distance` in map units to signal strength percentage.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Terminal.ResumeAireplay truncates signal strength like C# (int)"
)]
pub fn signal_strength(distance: f32) -> i32 {
    let distance_percent = f64::from(distance) * f64::from(100.0_f32) / f64::from(WIFI_RANGE);

    100 - distance_percent as i32
}

/// Derives a stable capture signal strength from a BSSID.
#[must_use]
pub fn wifi_power(bssid: &str) -> i32 {
    let sum: u32 = bssid.bytes().map(u32::from).sum();

    10 + i32::try_from(sum % 86).expect("the remainder is below 86")
}

/// Returns the ACK count required for `aircrack` to recover a key at signal `power`.
#[must_use]
pub const fn required_acks(power: i32) -> i32 {
    300_000 / (power + 15)
}

/// Reads packets from signal `power` every half second until the ACK count exceeds `max_acks`
/// (`Terminal.CollectData`). Each read adds up to 1.5 × power ACKs, then 25–249 packets. Returns the
/// packets read and the ACKs got.
pub fn collect_acks(rng: &mut Rng, power: i32, max_acks: i32) -> (i32, i32) {
    let (mut packets, mut acks) = (0_i32, 0_i32);
    let per_read = u32::try_from(power * 150 / 100).unwrap_or(0);

    loop {
        acks = acks.saturating_add(i32::try_from(rng.range(0, per_read)).unwrap_or(i32::MAX));

        if acks > max_acks || packets == i32::MAX {
            return (packets, acks);
        }

        packets = packets.saturating_add(i32::try_from(rng.range(25, 249)).unwrap_or(i32::MAX));
    }
}
