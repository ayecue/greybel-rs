//! Service control by scripts (`ServicesIntrinsics`): installing, starting, and stopping a service
//! as root.

use greyhack_behaviour::{DeviceKind, FIRST_VERSION, LIBRARY_FOLDER, ROOT_USER, ServiceId};
use greyhack_world_core::{ComputerId, Result};

/// Why a service operation is refused (`ServicesIntrinsics`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceControlFailure {
    /// Root privileges are required.
    RootOnly,
    /// HTTP is not installed, started, or stopped on routers.
    RouterHttp,
}

/// What starting a service did (`ServicesIntrinsics.start_service`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStart {
    /// The computer no longer exists.
    Missing,
    /// The service started.
    Success,
    /// The service did not start.
    Failure,
}

/// Methods of [`World`] for services control.
impl ServiceControl {
    pub async fn control_install_service(
        &self,
        computer: ComputerId,
        user: &str,
        service: ServiceId,
    ) -> Result<std::result::Result<bool, ServiceControlFailure>> {
        if let Err(failure) = self.check_service_control(computer, user, service).await? {
            return Ok(Err(failure));
        }

        let version = self.service_client_version(computer, service).await?;

        Ok(Ok(self
            .installs
            .install_service(computer, service, &version)
            .await?))
    }

    pub async fn control_start_service(
        &self,
        computer: ComputerId,
        user: &str,
        service: ServiceId,
    ) -> Result<std::result::Result<ServiceStart, ServiceControlFailure>> {
        if let Err(failure) = self.check_service_control(computer, user, service).await? {
            return Ok(Err(failure));
        }

        let version = self.service_client_version(computer, service).await?;

        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Ok(ServiceStart::Missing));
        }

        Ok(Ok(
            if self
                .computers
                .processes
                .start_service(computer, service, &version)
                .await?
            {
                ServiceStart::Success
            } else {
                ServiceStart::Failure
            },
        ))
    }

    pub async fn control_stop_service(
        &self,
        computer: ComputerId,
        user: &str,
        service: ServiceId,
    ) -> Result<std::result::Result<bool, ServiceControlFailure>> {
        if let Err(failure) = self.check_service_control(computer, user, service).await? {
            return Ok(Err(failure));
        }

        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Ok(false));
        }

        Ok(Ok(self
            .computers
            .processes
            .stop_service(computer, service)
            .await?))
    }
}

impl ServiceControl {
    pub async fn check_service_control(
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
                .networks
                .topology
                .computer(computer)
                .await?
                .is_some_and(|computer| computer.kind == DeviceKind::Router);

        Ok(if router_http {
            Err(ServiceControlFailure::RouterHttp)
        } else {
            Ok(())
        })
    }

    pub async fn service_client_version(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<String> {
        let path = format!("{LIBRARY_FOLDER}/{}", service.library());

        Ok(self
            .files
            .libraries
            .file_library(computer, &path, true)
            .await?
            .map_or_else(|| FIRST_VERSION.to_owned(), |library| library.version))
    }
}

/// A world service with only its declared dependencies.
pub struct ServiceControl {
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) installs: greyhack_world_modules::Dep<crate::services::InstallService>,
}

impl ServiceControl {
    pub(crate) fn new(
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        installs: greyhack_world_modules::Dep<crate::services::InstallService>,
    ) -> Self {
        Self {
            computers,
            networks,
            files,
            installs,
        }
    }
}
