//! Guild and company-ownership rules: reputation levels, majority ownership, the guild ranking and
//! the Crown (`Guilds`, `StocksCompany`, `NetworkLan.GetRankingPoints`).

use mono_random::Random;

use crate::{NetworkKind, SECONDS_PER_DAY, SECONDS_PER_HOUR, WorldSeed};

/// Days a guild must lead the ranking before it has the Crown (`Guilds.CrownQualificationDays`).
pub const CROWN_QUALIFICATION_DAYS: u64 = 15;
/// Price of CrownShield (`Guilds.Guild.CrownShieldPrice`).
pub const CROWN_SHIELD_PRICE: u32 = 25_000;
/// Share of a company one holder needs to own it, in percent (`StocksCompany.TryGetMajorityOwner`).
pub const MAJORITY_PERCENT: f32 = 50.0;
/// Points factor of a company hit by a DDoS (`StocksCompany.DdosRankingPenaltyMultiplier`).
pub const DDOS_LEADERBOARD_FACTOR: f32 = 0.8;
/// Hours a DDoS lowers a company's ranking points (`StocksCompany.DdosRankingPenaltyHours`).
pub const DDOS_LEADERBOARD_PENALTY_HOURS: u64 = 24;
/// Seconds a DDoS lowers a company's ranking points.
pub const DDOS_LEADERBOARD_PENALTY_SECONDS: u64 = DDOS_LEADERBOARD_PENALTY_HOURS * SECONDS_PER_HOUR;
/// World-seed label of the share-price seed of a network that has none (`NetworkLan.seed`).
const COMPANY_SEED_LABEL: &str = "company-seed-v1";
/// Admin absence penalty a company starts with (`Guilds.Guild.AdminAbsenceIncidentPenaltyPercent`).
pub const ADMIN_ABSENCE_PENALTY_PERCENT: u32 = 50;
/// Percent the penalty recovers each day after a new administrator (`Guilds.Guild.AdminAbsenceDailyRecoveryPercent`).
pub const ADMIN_ABSENCE_DAILY_RECOVERY_PERCENT: u32 = 10;

const MAX_LEVEL: u32 = 15;
const LEVEL_EXP_FACTOR: u64 = 275;
const MAX_INVESTMENT_PERKS: u32 = 5;
const MAX_ADMIN_RESTORE_PERKS: u32 = 5;
const MIN_INVESTMENT_LEVEL: u32 = 5;

/// Guild level of `total_exp` experience (`Reputation.GetCurrentLevel`).
#[must_use]
pub fn reputation_level(total_exp: u32) -> u32 {
    if total_exp == 0 {
        return 0;
    }

    ((f64::from(total_exp) / LEVEL_EXP_FACTOR as f64)
        .sqrt()
        .floor()) as u32
}

/// Companies a guild may invest in with `perks` purchased investment licences
/// (`GuildLevelRules.GetMaxInvestedCompaniesForPurchasedPerks`).
#[must_use]
pub fn max_invested_companies_for_perks(perks: u32) -> u32 {
    perks.min(MAX_INVESTMENT_PERKS)
}

/// Companies a guild of `level` may invest in (`GuildLevelRules.GetMaxInvestedCompanies`).
#[must_use]
pub fn max_invested_companies_for_level(level: u32) -> u32 {
    if level < MIN_INVESTMENT_LEVEL {
        return 0;
    }

    let level = level.min(MAX_LEVEL);

    1 + (level - MIN_INVESTMENT_LEVEL) * 4 / 10
}

/// Companies counted for a guild's ranking (`Guilds.Guild.GetEffectiveMaxInvestedCompanies`).
#[must_use]
pub fn effective_max_invested_companies(perks: u32, total_exp: u32) -> u32 {
    max_invested_companies_for_perks(perks).min(max_invested_companies_for_level(reputation_level(
        total_exp,
    )))
}

/// Companies a guild may turn Admin Restore off for with `perks` purchased Admin Restore perks
/// (`GuildLevelRules.GetMaxAdminRestoreCompaniesForPurchasedPerks`).
#[must_use]
pub fn max_admin_restore_companies_for_perks(perks: u32) -> u32 {
    perks.min(MAX_ADMIN_RESTORE_PERKS)
}

/// Companies a guild of `level` may turn Admin Restore off for (`GuildLevelRules.GetMaxAdminRestoreCompanies`).
#[must_use]
pub fn max_admin_restore_companies_for_level(level: u32) -> u32 {
    max_invested_companies_for_level(level)
}

/// Companies a guild may have Admin Restore off for (`Guilds.Guild.GetEffectiveMaxAdminRestoreOffCompanies`).
#[must_use]
pub fn effective_max_admin_restore_disabled_companies(perks: u32, total_exp: u32) -> u32 {
    max_admin_restore_companies_for_perks(perks).min(max_admin_restore_companies_for_level(
        reputation_level(total_exp),
    ))
}

/// Shares a company issues (`NetworkLan.ConfigMaxDevices`).
#[must_use]
pub const fn max_stocks(kind: NetworkKind) -> u32 {
    match kind {
        NetworkKind::Police
        | NetworkKind::University
        | NetworkKind::Bank
        | NetworkKind::Neurobox => 1_000_000,
        NetworkKind::MailService
        | NetworkKind::HackShop
        | NetworkKind::ComputerShop
        | NetworkKind::CurrencyCreation => 500_000,
        _ => 100_000,
    }
}

/// Whether companies of `kind` are listed on the stock market (`StocksCompany.IsListedOnStockMarket`).
#[must_use]
pub const fn is_listed_on_stock_market(kind: NetworkKind) -> bool {
    matches!(
        kind,
        NetworkKind::Supermarket
            | NetworkKind::FastFood
            | NetworkKind::Garage
            | NetworkKind::MobileShop
            | NetworkKind::ComputerShop
            | NetworkKind::HardwareManufacturer
    )
}

/// Highest share price a company with average work performance code `work` can draw
/// (`NetworkLan.GetMaxPrice`).
#[must_use]
pub const fn max_share_price(work: u8) -> i32 {
    match work {
        0 => 1,
        1 => 3,
        2 => 5,
        3 => 20,
        4 => 50,
        5 => 100,
        _ => 0,
    }
}

/// Ranking points of a company issuing `max_stocks` shares at `share_price` with average work
/// performance code `work` (`NetworkLan.GetRankingPoints`).
#[must_use]
pub fn leaderboard_points(max_stocks: u32, share_price: i32, work: u8) -> i32 {
    let value = i128::from(max_stocks) * i128::from(share_price.max(1));
    let factor = 10 + i128::from(work) - 2;
    let product = value * factor;
    let quotient = product.div_euclid(10_000);
    let remainder = product.rem_euclid(10_000);
    let nearest = if remainder > 5_000 || (remainder == 5_000 && quotient % 2 != 0) {
        quotient + 1
    } else {
        quotient
    };

    i32::try_from(nearest.clamp(0, i128::from(i32::MAX))).unwrap_or(i32::MAX)
}

/// Largest holder of a company issuing `max_stocks` shares, when it owns more than half
/// (`StocksCompany.TryGetMajorityOwner`). `holders` lists holders in the order they first bought;
/// the first of equal holders wins. Returns its name and percent.
#[must_use]
pub fn majority_owner<'a>(
    max_stocks: u32,
    holders: impl IntoIterator<Item = (&'a str, u32)>,
) -> Option<(&'a str, f32)> {
    if max_stocks == 0 {
        return None;
    }

    let mut owner = None;
    let mut percent = 0.0_f32;

    for (name, shares) in holders {
        let share = shares as f32 / max_stocks as f32 * 100.0;

        if share > percent {
            owner = Some(name);
            percent = share;
        }
    }

    owner
        .filter(|_| percent > MAJORITY_PERCENT)
        .map(|name| (name, percent))
}

/// World second `days` days after `at` (`DateTime.AddDays`).
#[must_use]
pub const fn add_days(at: u64, days: u64) -> u64 {
    at + days * SECONDS_PER_DAY
}

/// World second the day of `at` ends (`DateTime.Date.AddDays(1)`).
#[must_use]
pub const fn next_day(at: u64) -> u64 {
    (at / SECONDS_PER_DAY + 1) * SECONDS_PER_DAY
}

/// Returns the share-price seed of a network at `public_ip` that has none (`NetworkLan.seed`).
#[must_use]
pub fn default_company_seed(world_seed: &WorldSeed, public_ip: &str) -> i32 {
    world_seed.derive_int(COMPANY_SEED_LABEL, &[public_ip])
}

/// Returns the share price of a company drawn from its network `seed` and average work performance
/// code `work` (`NetworkLan.GetPriceRecords`).
#[must_use]
pub fn company_share_price(seed: i32, work: u8) -> i32 {
    let mut random = Random::new(seed);
    let first = random.next_below(usize::try_from(max_share_price(work)).unwrap_or(0));
    let second = random.next_below(11) as i32 - 5;

    (first as i32 + second).abs().max(1)
}
