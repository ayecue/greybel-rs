//! NPC job performance (`NpcInfo.WorkStatus`).

/// NPC job performance (`NpcInfo.WorkStatus`), ordered worst first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkPerformance {
    VeryPoor = 0,
    Poor = 1,
    Normal = 2,
    Good = 3,
    VeryGood = 4,
    Excellent = 5,
}

impl WorkPerformance {
    /// All performance levels in Grey Hack order.
    pub const ALL: [Self; 6] = [
        Self::VeryPoor,
        Self::Poor,
        Self::Normal,
        Self::Good,
        Self::VeryGood,
        Self::Excellent,
    ];

    /// Returns the performance level numbered `code` in Grey Hack order.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get(usize::from(code)).copied()
    }
}

/// Returns the average work performance code of a network's `total` NPCs, counting only the codes
/// of those who `available` for work (`NetworkLan.GetWorkPerformance`); 0 without NPCs.
#[must_use]
pub fn average_work_performance(
    total: usize,
    available: impl IntoIterator<Item = WorkPerformance>,
) -> u8 {
    if total == 0 {
        return 0;
    }

    let sum: usize = available
        .into_iter()
        .map(|performance| performance as usize)
        .sum();

    u8::try_from(sum / total).unwrap_or(u8::MAX)
}
