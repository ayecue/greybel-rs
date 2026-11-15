//! Computer processes, their services, rebooting, and what scripts do to them.

use std::rc::Rc;

use greyhack_world_modules::Dep;

use crate::WorldRef;
use crate::storage::{
    ComputerRepository, HardwareRepository, LibraryIdentityRepository, PersonRepository,
    UserRepository,
};
use crate::systems::{FileLookup, FilePermissionService, NetCardService, Startup, UserService};

pub mod reboot;
pub mod scripts;
pub mod services;
pub mod table;

/// Computer process, service, and reboot rules.
pub struct ProcessService {
    pub(crate) world: WorldRef,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) hardware_rows: Rc<HardwareRepository>,
    pub(crate) users: Dep<UserService>,
    pub(crate) startup: Dep<Startup>,
    pub(crate) net_cards: Dep<NetCardService>,
    pub(crate) library_rows: Rc<LibraryIdentityRepository>,
    pub(crate) user_rows: Rc<UserRepository>,
    pub(crate) person_rows: Rc<PersonRepository>,
    pub(crate) lookup: Dep<FileLookup>,
    pub(crate) permissions: Dep<FilePermissionService>,
}

impl ProcessService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        hardware_rows: Rc<HardwareRepository>,
        library_rows: Rc<LibraryIdentityRepository>,
        user_rows: Rc<UserRepository>,
        person_rows: Rc<PersonRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            computer_rows,
            hardware_rows,
            users: registry.get(),
            startup: registry.get(),
            net_cards: registry.get(),
            library_rows,
            user_rows,
            person_rows,
            lookup: registry.get(),
            permissions: registry.get(),
        }
    }
}
