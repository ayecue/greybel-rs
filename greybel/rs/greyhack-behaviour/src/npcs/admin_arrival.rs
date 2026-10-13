//! What a newly hired administrator does to a guild's penalty for a company.

use crate::AdminAbsence;

/// What a newly hired administrator does to the penalty of `current` on `today`
/// (`Guilds.Guild.MarkCompanyAdminHired`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminArrival {
    /// The record stays as it is.
    Same,
    /// The record is removed.
    Remove,
    /// The record is replaced and starts to recover.
    Recover(AdminAbsence),
}

/// See [`AdminArrival`].
#[must_use]
pub fn admin_arrives(current: Option<AdminAbsence>, today: u64) -> AdminArrival {
    let Some(current) = current.filter(|record| !record.recovering) else {
        return AdminArrival::Same;
    };
    let percent = current.percent_on(today);

    if percent == 0 {
        return AdminArrival::Remove;
    }

    AdminArrival::Recover(AdminAbsence {
        percent,
        recovery_day: Some(today),
        recovering: true,
    })
}
