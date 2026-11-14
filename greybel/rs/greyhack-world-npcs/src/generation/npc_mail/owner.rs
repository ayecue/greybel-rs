use super::Colleague;
use crate::CompanyAlign;

/// The NPC whose mailbox is generated.
#[derive(Debug, Clone)]
pub(crate) struct Owner {
    pub colleague: Colleague,
    /// Seconds from midnight at which the break starts
    /// (`NPCSchedule.GetActivityTimeStart(Activity.DescansoBreve)`).
    pub break_start: u64,
    pub company_align: CompanyAlign,
    /// Mail address of the zero-day contact (`NpcInfo.GetDayZeroContact`).
    pub day_zero_contact: Option<String>,
}
