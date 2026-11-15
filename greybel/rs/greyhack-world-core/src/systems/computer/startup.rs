//! Computer startup checks (`Computer.CheckInicio`) and library filename validation
//! (`Library.IsValidLibrary`).

use std::rc::Rc;

use greyhack_behaviour::{
    DeviceKind, FileKind, ROUTER_STARTUP_FILES, STARTUP_FILES, StartupContent, StartupRole,
};

use crate::storage::*;
use crate::systems::{Startup as StartupService, *};
use crate::*;

impl StartupService {
    /// Returns startup failures for `computer` in file-check order (`Computer.CheckInicio`),
    /// following symlinks. Routers and switches check their own files; hubs have no file system and
    /// no failures. Empty result means startup succeeds.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn startup_failures(&self, computer: ComputerId) -> Result<Vec<StartupFailure>> {
        let Some(row) = self.computer_rows.computer(computer).await? else {
            return Ok(Vec::new());
        };

        let files: &[(&'static str, StartupRole)] = match row.kind {
            DeviceKind::Hub => return Ok(Vec::new()),
            DeviceKind::Router | DeviceKind::Switch => &ROUTER_STARTUP_FILES,
            _ => &STARTUP_FILES,
        };
        let mut failures = Vec::new();

        for &(path, role) in files {
            let Some(file) = self.lookup.file(computer, path, true).await? else {
                failures.push(match role {
                    StartupRole::Kernel => StartupFailure::MissingKernelFile(path),
                    StartupRole::Graphics => StartupFailure::MissingGraphicsFile(path),
                    StartupRole::System => StartupFailure::MissingSystemFile(path),
                });
                continue;
            };
            let intact = match StartupContent::of(path) {
                StartupContent::Library => self.is_valid_library(&file).await?,
                StartupContent::Text => !file.data().is_some_and(|data| data.is_binary()),
                StartupContent::Binary => file.data().is_some_and(|data| data.is_binary()),
            };

            if !intact {
                failures.push(StartupFailure::InvalidFile(path));
            }
        }

        Ok(failures)
    }

    /// Returns whether `file` contains the library named by its filename
    /// (`Library.IsValidLibrary`). Renaming another library does not make it valid.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_valid_library(&self, file: &Node) -> Result<bool> {
        if file
            .data()
            .is_none_or(|data| data.kind != FileKind::Library)
        {
            return Ok(false);
        }

        Ok(self
            .library_files
            .library_of(file)
            .await?
            .is_some_and(|library| library.name() == file.name()))
    }
}

/// Computer startup validation.
pub struct Startup {
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) lookup: Dep<FileLookup>,
    pub(crate) library_files: Dep<LibraryFileService>,
}

impl Startup {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
    ) -> Self {
        Self {
            computer_rows,
            lookup: registry.get(),
            library_files: registry.get(),
        }
    }
}
