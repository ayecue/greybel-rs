//! Computer accounts: sessions, stored accounts, and what scripts do to them.

use std::rc::Rc;

use crate::storage::*;
use crate::*;

pub mod accounts;
pub mod script_accounts;
pub mod sessions;

/// Computer users, groups, passwords, and sessions.
pub struct UserService {
    pub(crate) world: WorldRef,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) user_rows: Rc<UserRepository>,
    pub(crate) account_files: Dep<dyn crate::AccountFileService>,
    pub network_admins: Dep<dyn crate::NetworkAdminLookup>,
    pub npcs: Dep<dyn crate::NpcState>,
}

impl UserService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        user_rows: Rc<UserRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            computer_rows,
            user_rows,
            account_files: registry.get(),
            network_admins: registry.get(),
            npcs: registry.get(),
        }
    }
}
