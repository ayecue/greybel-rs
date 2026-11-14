use std::collections::HashMap;

use mono_random::Random;

use super::pending_service::PendingService;
use super::pending_system_log::PendingSystemLog;
use crate::model::Mission;

/// Everything `ResolveRuntimeTags` changes besides the tags it returns.
pub(super) struct State<'r> {
    pub(super) mission: Mission,
    pub(super) npc_by_beat: HashMap<i32, i32>,
    pub(super) global_npc: i32,
    pub(super) global_string: Option<String>,
    pub(super) services: Vec<PendingService>,
    pub(super) logs: Vec<PendingSystemLog>,
    pub(super) random: &'r mut Random,
}
