use crate::{JobRole, WorkPerformance};

/// An NPC of the network (`NpcInfo`) as the mails read it.
#[derive(Debug, Clone)]
pub(crate) struct Colleague {
    /// Account name (`NpcInfo.GetSurname`), the key of `NetworkLan.npcInfos`.
    pub surname: String,
    /// First name (`NpcInfo.GetName`).
    pub first_name: String,
    /// Mail address (`NpcInfo.GetMailAddress`).
    pub address: String,
    pub job_role: JobRole,
    pub work_performance: WorkPerformance,
}
