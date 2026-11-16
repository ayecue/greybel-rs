//! A company that counts towards a guild's ranking.

use crate::DDOS_LEADERBOARD_FACTOR;

/// A company that counts towards a guild's ranking, in the order of the game's ranking query.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedCompany {
    /// Points the company has, before any penalty.
    pub points: i32,
    /// Percent of the company the guild's bank account owns.
    pub owner_percent: f32,
    /// Whether a DDoS lowers the company's points.
    pub ddos_penalty: bool,
    /// Percent its missing administrator costs.
    pub admin_penalty_percent: u32,
}

/// Points a guild gets for `companies`, of which only the first `max_companies` count
/// (`Database.GetGuildRanking`).
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Database.GetGuildRanking uses C# float penalties and Math.Round for guild points"
)]
pub fn guild_points(companies: &[RankedCompany], max_companies: u32) -> i64 {
    let mut total = 0_i64;

    for (index, company) in companies.iter().enumerate() {
        if u32::try_from(index).expect("value fits the target integer type") >= max_companies {
            break;
        }

        let points = company.points.max(0) as f32;
        let mut after_penalties = if company.ddos_penalty {
            points * DDOS_LEADERBOARD_FACTOR
        } else {
            points
        };

        if company.admin_penalty_percent > 0 {
            after_penalties *= (100 - company.admin_penalty_percent.min(100)) as f32 / 100.0;
        }

        let percent = company.owner_percent.clamp(0.0, 100.0);

        let points_with_owner =
            f64::from(after_penalties) * (f64::from(percent) / f64::from(100.0_f32));

        total += points_with_owner.round_ties_even() as i64;
    }

    total
}

/// Returns the weight by which the ranking orders a company of `points` ranking points owned by a
/// guild to `percent` percent (`Database.GetGuildRanking`).
#[must_use]
pub fn company_weight(points: i32, percent: f32) -> f64 {
    f64::from(points) * f64::from(percent)
}

/// Orders the companies of a guild, each with its [`company_weight`] and name, heaviest first and
/// by name among equals (`Database.GetGuildRanking`).
#[must_use]
pub fn order_companies(mut companies: Vec<(f64, String, RankedCompany)>) -> Vec<RankedCompany> {
    companies.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    companies
        .into_iter()
        .map(|(_, _, company)| company)
        .collect()
}
