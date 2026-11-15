//! What scripts do to files through `FileIntrinsics`: reading, writing, creating, copying, moving, and
//! deleting them.

use std::rc::Rc;

use crate::storage::*;
use crate::systems::*;
use crate::*;

pub mod content;
pub mod create;
pub mod failure;
pub mod transfer;

/// File operations exposed to scripts.
pub struct ScriptFileService {
    pub(crate) world: WorldRef,
    pub(crate) users: Dep<UserService>,
    pub(crate) changes: Dep<FileChangeService>,
    pub(crate) lookup: Dep<FileLookup>,
    pub(crate) permissions: Dep<FilePermissionService>,
    pub(crate) sizes: Dep<FileSizeService>,
    pub(crate) logs: Dep<LogService>,
    pub traces: Dep<dyn crate::PlayerTraceService>,
    pub(crate) computer_rows: Rc<ComputerRepository>,
}

impl ScriptFileService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            traces: registry.get(),
            users: registry.get(),
            changes: registry.get(),
            lookup: registry.get(),
            permissions: registry.get(),
            sizes: registry.get(),
            logs: registry.get(),
            computer_rows,
        }
    }
}
