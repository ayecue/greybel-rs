//! CTF ratings and the rewards they pay (`CtfHelperServer`).

use std::ops::RangeInclusive;

/// Marks a player or creator can give an event for quality and difficulty.
pub const MARK_RANGE: RangeInclusive<i64> = 1..=5;

/// Rounds halves to even, matching .NET `Math.Round`.
#[must_use]
pub fn round_even(value: f64) -> i64 {
    value.round_ties_even() as i64
}

/// Returns the mean quality mark above the lowest one of an event rated by `votes` players whose
/// quality marks add up to `quality_rank`.
#[must_use]
pub fn quality_score(quality_rank: i64, votes: i64) -> f32 {
    (quality_rank - votes) as f32 / votes as f32
}

/// Returns the dollars a player earns for rating an event whose marks add up to `quality_rank` and
/// `difficulty_rank` over `votes` votes before theirs.
#[must_use]
pub fn player_reward(quality_rank: i64, difficulty_rank: i64, votes: i64) -> i64 {
    round_even(f64::from(
        (quality_score(quality_rank, votes) / 4.0 + 1.0)
            * (difficulty_rank as f32 / votes as f32)
            * 150.0,
    ))
}

/// Returns the dollars the creator of an event earns for a rating, from their `creator_score` and
/// the event's `quality_rank` over `votes` votes including the new one.
#[must_use]
pub fn creator_reward(creator_score: f32, quality_rank: i64, votes: i64) -> i64 {
    round_even(f64::from(
        creator_score * quality_score(quality_rank, votes) * 10.0,
    ))
}

/// Returns the mean quality score of a creator's events, given each one's quality marks and votes;
/// 0 without events.
#[must_use]
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
