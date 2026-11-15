//! Camera web access (`NetworkHelperServer.AccessWebServerRpc`).

use greyhack_behaviour::ROOT_USER;
use greyhack_world_core::{ComputerId, Result};

use crate::{CameraFeed, CameraNpc};

/// Camera web-access errors (`NetworkHelperServer.AccessWebServerRpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CctvFailure {
    /// The password is invalid or the computer is not a camera.
    WrongPassword,
    /// A broken network appliance has disabled external camera access.
    NoExternalAccess,
}

impl Cctv {
    /// Returns a camera feed, a password prompt, or the reason access was refused.
    pub async fn access_cctv(
        &self,
        computer: ComputerId,
        password: Option<&str>,
    ) -> Result<Result<Option<CameraFeed>, CctvFailure>> {
        let Some(camera) = self.networks.topology.computer(computer).await? else {
            return Ok(Err(CctvFailure::WrongPassword));
        };

        if camera.kind != greyhack_behaviour::DeviceKind::Camera {
            return Ok(Err(CctvFailure::WrongPassword));
        }

        let Some(password) = password else {
            return Ok(Ok(None));
        };
        let users = self.computers.users.users(computer).await?;
        let user = users
            .iter()
            .find(|user| user.name == "cctv")
            .or_else(|| users.iter().find(|user| user.name == ROOT_USER));
        let Some(user) = user else {
            return Ok(Err(CctvFailure::WrongPassword));
        };

        if user.password != password {
            return Ok(Err(CctvFailure::WrongPassword));
        }

        let mut npcs = Vec::new();

        if let Some(router) = self.networks.topology.router_of(computer).await? {
            for device in self.networks.topology.devices(router.id).await? {
                for npc in self.npcs.npcs_on(device).await? {
                    let name = self
                        .npcs
                        .person(npc.person)
                        .await?
                        .map(|person| person.name)
                        .unwrap_or(npc.name);

                    npcs.push(CameraNpc {
                        name,
                        online: npc.online,
                    });
                }

                if self
                    .smart_devices
                    .smart(device)
                    .await?
                    .is_some_and(|smart| smart.is_broken(self.world.time_seconds()))
                {
                    return Ok(Err(CctvFailure::NoExternalAccess));
                }
            }
        }

        Ok(Ok(Some(CameraFeed {
            local_ip: camera.local_ip,
            npcs,
        })))
    }
}

/// Camera web access and the people visible on a camera's network.
pub struct Cctv {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) smart_devices: std::rc::Rc<crate::storage::SmartDeviceRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
}

impl Cctv {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        smart_devices: std::rc::Rc<crate::storage::SmartDeviceRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
    ) -> Self {
        Self {
            world: world.clone(),
            smart_devices,
            computers,
            networks,
            npcs,
        }
    }
}
