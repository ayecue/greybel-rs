//! A guild's place in the ranking.

/// A guild's place in the ranking (`Guilds.GuildRankingEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedGuild {
    pub name: String,
    pub total_points: i64,
}

/// Sorts `guilds` by points, then name, as the game ranks them.
pub fn sort_leaderboard(guilds: &mut [RankedGuild]) {
    guilds.sort_by(|a, b| {
        b.total_points
            .cmp(&a.total_points)
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// The guild leading `ranking`, when it has no tie for first (`Guilds.EvaluateCrownRanking`).
#[must_use]
pub fn crown_leader(leaderboard: &[RankedGuild]) -> Option<&RankedGuild> {
    match leaderboard {
        [only] => Some(only),
        [first, second, ..] if first.total_points != second.total_points => Some(first),
        _ => None,
    }
}
