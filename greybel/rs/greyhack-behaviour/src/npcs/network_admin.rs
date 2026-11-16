//! Network administrators (`NetworkLan.adminInfo`).

use crate::{Rng, WorkPerformance};

/// Average of the intelligence and computer experience from which an administrator is a super
/// administrator (`NpcInfo.IsSuperAdmin`).
const SUPER_ADMIN_AVERAGE: f32 = 8.0;

/// Network administrator (`NetworkLan.adminInfo`): the NPC with the admin role and the attributes
/// used to intercept attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkAdmin {
    /// Intelligence from 0 to 10 (`NpcInfo.inteligencia`).
    pub intelligence: u8,
    /// Computer experience from 1 to 10 (`NpcInfo.expPcs`).
    pub pc_experience: u8,
    pub work_performance: WorkPerformance,
    /// Whether the administrator is dead or fired (`NpcInfo.IsDeadOrFired`).
    pub dead_or_fired: bool,
}

impl NetworkAdmin {
    /// Draws the job statistics of an administrator (`NpcInfo.NpcInfo`).
    pub fn draw(rng: &mut Rng) -> Self {
        Self {
            intelligence: u8::try_from(rng.range(0, 10))
                .expect("value fits the target integer type"),
            pc_experience: u8::try_from(rng.range(1, 10))
                .expect("value fits the target integer type"),
            work_performance: *rng.pick(&WorkPerformance::ALL),
            dead_or_fired: false,
        }
    }

    /// Returns whether the administrator forces attacks against network computers to use guest
    /// (`NpcInfo.IsSuperAdmin` with `checkDeadOrFired`). Requires above-normal job performance,
    /// average intelligence and computer experience of at least 8, active work status, and neither
    /// dead nor fired.
    #[must_use]
    pub fn is_super_admin(&self) -> bool {
        self.is_super_admin_by_stats() && !self.dead_or_fired
    }

    /// Returns whether the statistics are those of a super administrator (`NpcInfo.IsSuperAdmin`
    /// without `checkDeadOrFired`).
    #[must_use]
    pub fn is_super_admin_by_stats(&self) -> bool {
        self.work_performance > WorkPerformance::Normal
            && f64::from(self.intelligence + self.pc_experience) / f64::from(2.0_f32)
                >= f64::from(SUPER_ADMIN_AVERAGE)
    }
}
