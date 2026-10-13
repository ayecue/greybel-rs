//! Computer hardware, ports, and library versions.

use crate::model::{ComputerId, Hardware, Port};
use crate::{Result, World};

impl World {
    /// Returns every part installed in `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn hardware(&self, computer: ComputerId) -> Result<Hardware> {
        self.db.hardware(computer).await
    }

    /// Returns `computer` port `number`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port(&self, computer: ComputerId, number: u16) -> Result<Option<Port>> {
        self.db.port(computer, number).await
    }

    /// Changes library file `path` on `computer` to `version`, preserving kind and weaknesses, and
    /// returns whether it was a library. Service files and versions correspond to
    /// `LanPcInfo.libVersions` (`RedGlobal.ShouldRestore`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_library_version(
        &mut self,
        computer: ComputerId,
        path: &str,
        version: &str,
    ) -> Result<bool> {
        let Some(node) = self.file(computer, path, false).await? else {
            return Ok(false);
        };
        let Some(mut library) = self.library_of(&node).await? else {
            return Ok(false);
        };

        library.version = version.to_owned();
        self.set_library_of(&node, Some(&library)).await?;

        Ok(true)
    }

    /// Changes `computer` port `number` to its service library at `version` and returns whether the
    /// port exists. Grey Hack associates each service with a versioned client library
    /// (`RedGlobal.ShouldRestore`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_port_library_version(
        &mut self,
        computer: ComputerId,
        number: u16,
        version: &str,
    ) -> Result<bool> {
        let Some(port) = self.port(computer, number).await? else {
            return Ok(false);
        };
        let library = self
            .db
            .library_row(port.service.library_kind(), version)
            .await?;

        self.db.set_port_library(computer, number, library).await
    }
}
