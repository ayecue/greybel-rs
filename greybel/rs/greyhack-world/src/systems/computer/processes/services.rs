//! Services of a computer and their ports.

use greyhack_behaviour::{ROOT_USER, ServiceId};

use crate::{ComputerId, Port, Result, World};

impl World {
    /// Returns `computer` ports in opening order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn ports(&self, computer: ComputerId) -> Result<Vec<Port>> {
        self.db.ports(computer).await
    }

    /// Returns whether `computer` runs service `id` with an open port.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn has_service(&self, computer: ComputerId, id: ServiceId) -> Result<bool> {
        Ok(self.db.service_port(computer, id).await?.is_some())
    }

    /// Starts service `id` at `version` by opening its port and spawning its daemon. Returns
    /// whether it newly started.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn start_service(
        &mut self,
        computer: ComputerId,
        id: ServiceId,
        version: &str,
    ) -> Result<bool> {
        if self.has_service(computer, id).await? {
            return Ok(false);
        }

        let library = self.db.library_row(id.library_kind(), version).await?;

        self.db
            .insert_port(
                computer,
                &Port {
                    number: id.default_port(),
                    service: id,
                    version: version.to_owned(),
                    closed: false,
                    visible: true,
                },
                library,
            )
            .await?;
        self.db
            .spawn_process(computer, ROOT_USER, id.daemon(), Some(id), false)
            .await?;

        Ok(true)
    }

    /// Stops service `id` by closing its port and killing its daemon. Returns whether it was
    /// running.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn stop_service(&mut self, computer: ComputerId, id: ServiceId) -> Result<bool> {
        let had = self.has_service(computer, id).await?;

        self.db.remove_service_ports(computer, id).await?;
        self.process_ends.insert(computer);
        self.db.end_service_processes(computer, id).await?;

        Ok(had)
    }

    /// Returns the port of the service on `computer` that runs library `library_name`, or 0 when none does; the last
    /// such service wins (`ScriptsUtil.GetPuertoServicio`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn library_service_port(
        &self,
        computer: ComputerId,
        library_name: &str,
    ) -> Result<u16> {
        Ok(self
            .ports(computer)
            .await?
            .into_iter()
            .rev()
            .find(|port| port.service.library() == library_name)
            .map_or(0, |port| port.number))
    }
}
