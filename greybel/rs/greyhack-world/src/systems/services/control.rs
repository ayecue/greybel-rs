//! Service control by scripts (`ServiceIntrinsics`): installing, starting, and stopping a service
//! as root.

use greyhack_behaviour::{DeviceKind, FIRST_VERSION, LIBRARY_FOLDER, ROOT_USER, ServiceId};

use crate::{ComputerId, Result, World};

/// Why a service operation is refused (`ServiceIntrinsics`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceControlFailure {
    /// Root privileges are required.
    RootOnly,
    /// HTTP is not installed, started, or stopped on routers.
    RouterHttp,
}

/// What starting a service did (`ServiceIntrinsics.start_service`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStart {
    /// The computer is gone.
    Gone,
    /// The service started.
    Success,
    /// The service did not start.
    Failure,
}

impl World {
    /// Rejects non-root `user` and HTTP service control on routers.
    async fn check_service_control(
        &self,
        computer: ComputerId,
        user: &str,
        service: ServiceId,
    ) -> Result<std::result::Result<(), ServiceControlFailure>> {
        if user != ROOT_USER {
            return Ok(Err(ServiceControlFailure::RootOnly));
        }

        let router_http = service == ServiceId::Http
            && self
                .computer(computer)
                .await?
                .is_some_and(|computer| computer.kind == DeviceKind::Router);

        Ok(if router_http {
            Err(ServiceControlFailure::RouterHttp)
        } else {
            Ok(())
        })
    }

    /// Reads the service version from its client library.
    async fn service_client_version(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<String> {
        let path = format!("{LIBRARY_FOLDER}/{}", service.library());

        Ok(self
            .file_library(computer, &path, true)
            .await?
            .map_or_else(|| FIRST_VERSION.to_owned(), |library| library.version))
    }

    /// Installs `service` using its client library version (`ServiceIntrinsics.install_service`).
    /// Returns whether it installed.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn control_install_service(
        &mut self,
        computer: ComputerId,
        user: &str,
        service: ServiceId,
    ) -> Result<std::result::Result<bool, ServiceControlFailure>> {
        if let Err(failure) = self.check_service_control(computer, user, service).await? {
            return Ok(Err(failure));
        }

        let version = self.service_client_version(computer, service).await?;

        Ok(Ok(self
            .install_service(computer, service, &version)
            .await?))
    }

    /// Opens the port of `service` and starts its daemon using the client library version
    /// (`ServiceIntrinsics.start_service`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn control_start_service(
        &mut self,
        computer: ComputerId,
        user: &str,
        service: ServiceId,
    ) -> Result<std::result::Result<ServiceStart, ServiceControlFailure>> {
        if let Err(failure) = self.check_service_control(computer, user, service).await? {
            return Ok(Err(failure));
        }

        let version = self.service_client_version(computer, service).await?;

        if self.computer(computer).await?.is_none() {
            return Ok(Ok(ServiceStart::Gone));
        }

        Ok(Ok(
            if self.start_service(computer, service, &version).await? {
                ServiceStart::Success
            } else {
                ServiceStart::Failure
            },
        ))
    }

    /// Stops the daemon of `service` and closes its port (`ServiceIntrinsics.stop_service`). Returns
    /// whether it stopped.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn control_stop_service(
        &mut self,
        computer: ComputerId,
        user: &str,
        service: ServiceId,
    ) -> Result<std::result::Result<bool, ServiceControlFailure>> {
        if let Err(failure) = self.check_service_control(computer, user, service).await? {
            return Ok(Err(failure));
        }

        if self.computer(computer).await?.is_none() {
            return Ok(Ok(false));
        }

        Ok(Ok(self.stop_service(computer, service).await?))
    }
}

#[cfg(test)]
#[path = "tests/control.rs"]
mod tests;
