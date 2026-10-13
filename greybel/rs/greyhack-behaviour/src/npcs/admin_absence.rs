//! A guild's penalty for a company whose administrator is missing (`Guilds.AdminAbsencePenalty`).

use crate::{ADMIN_ABSENCE_DAILY_RECOVERY_PERCENT, ADMIN_ABSENCE_PENALTY_PERCENT};

/// A guild's penalty for a company whose administrator is missing (`Guilds.AdminAbsencePenalty`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminAbsence {
    pub percent: u32,
    /// World day the recovery started, while recovering.
    pub recovery_day: Option<u64>,
    pub recovering: bool,
}

impl AdminAbsence {
    /// Penalty percent on world day `today` (`Guilds.Guild.GetCompanyAdminPenaltyPercent`).
    #[must_use]
    pub fn percent_on(&self, today: u64) -> u32 {
        let Some(start) = self.recovery_day.filter(|_| self.recovering) else {
            return self.percent;
        };

        if today <= start {
            return self.percent;
        }

        let recovery_percent =
            (today - start).saturating_mul(u64::from(ADMIN_ABSENCE_DAILY_RECOVERY_PERCENT));

        u32::try_from(u64::from(self.percent).saturating_sub(recovery_percent)).unwrap_or(0)
    }
}

/// The penalty after the administrator of a company goes missing on `today`; `None` means the
/// record is unchanged (`Guilds.Guild.MarkCompanyAdminMissing`). Returns the new record.
#[must_use]
pub fn admin_absent(current: Option<AdminAbsence>, today: u64) -> Option<AdminAbsence> {
    let Some(current) = current else {
        return Some(AdminAbsence {
            percent: ADMIN_ABSENCE_PENALTY_PERCENT,
            recovery_day: None,
            recovering: false,
        });
    };

    if !current.recovering {
        return None;
    }

    Some(AdminAbsence {
        percent: current.percent_on(today).max(ADMIN_ABSENCE_PENALTY_PERCENT),
        recovery_day: None,
        recovering: false,
    })
}
