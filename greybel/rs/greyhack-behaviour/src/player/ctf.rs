//! CTF ratings and the rewards they pay (`CtfHelperServer`).

use std::ops::RangeInclusive;

/// Marks a player or creator can give an event for quality and difficulty.
pub const MARK_RANGE: RangeInclusive<i64> = 1..=5;

/// Rounds halves to even, matching .NET `Math.Round`.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "CTF_Event rating methods truncate Math.Round results like C# (int)"
)]
pub fn round_even(value: f64) -> i64 {
    value.round_ties_even() as i64
}

/// Returns the mean quality mark above the lowest one of an event rated by `votes` players whose
/// quality marks add up to `quality_rank`.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "CTF_Event quality calculations use C# floats"
)]
pub fn quality_score(quality_rank: i64, votes: i64) -> f32 {
    (quality_rank - votes) as f32 / votes as f32
}

/// Returns the dollars a player earns for rating an event whose marks add up to `quality_rank` and
/// `difficulty_rank` over `votes` votes before theirs.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "CTF_Event.AddScore computes player rewards with C# floats"
)]
pub fn player_reward(quality_rank: i64, difficulty_rank: i64, votes: i64) -> i64 {
    let quality = quality_ratio(quality_rank, votes);
    let difficulty = f64::from(difficulty_rank as f32) / f64::from(votes as f32);

    round_even(
        (quality / f64::from(4.0_f32) + f64::from(1.0_f32)) * difficulty * f64::from(150.0_f32),
    )
}

/// The quotient used directly inside the reward expressions, before any C# float local stores it.
#[expect(
    clippy::cast_precision_loss,
    reason = "C# casts both ranking operands to float inside the reward expressions"
)]
fn quality_ratio(quality_rank: i64, votes: i64) -> f64 {
    f64::from((quality_rank - votes) as f32) / f64::from(votes as f32)
}

/// Returns the dollars the creator of an event earns for a rating, from their `creator_score` and
/// the event's `quality_rank` over `votes` votes including the new one.
#[must_use]
pub fn creator_reward(creator_score: f32, quality_rank: i64, votes: i64) -> i64 {
    round_even(f64::from(creator_score) * quality_ratio(quality_rank, votes) * f64::from(10.0_f32))
}

/// Returns the mean quality score of a creator's events, given each one's quality marks and votes;
/// 0 without events.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "CTF_Event.GetCreatorScore computes the mean with C# floats"
)]
pub fn creator_score(events: &[(i64, i64)]) -> f32 {
    if events.is_empty() {
        return 0.0;
    }

    let total: f32 = events
        .iter()
        .map(|&(quality_rank, votes)| quality_score(quality_rank, votes))
        .sum();

    total / events.len() as f32
}
