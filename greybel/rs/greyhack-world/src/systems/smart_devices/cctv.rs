//! Camera web access (`NetworkHelperServer.AccessWebServerRpc`).

use greyhack_behaviour::ROOT_USER;

use crate::{CameraFeed, CameraNpc, ComputerId, Result, World};

/// Camera web-access errors (`NetworkHelperServer.AccessWebServerRpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CctvFailure {
    /// The password is invalid or the computer is not a camera.
    WrongPassword,
    /// A broken network appliance has disabled external camera access.
    NoExternalAccess,
}

impl World {
    /// Authenticates CCTV web access and returns its feed. `None` requests a password; wrong
    /// passwords and accident lockout return Grey Hack errors
    /// (`NetworkHelperServer.AccessWebServerRpc`).
    pub async fn access_cctv(
        &self,
        computer: ComputerId,
        password: Option<&str>,
    ) -> Result<Result<Option<CameraFeed>, CctvFailure>> {
        let Some(camera) = self.computer(computer).await? else {
            return Ok(Err(CctvFailure::WrongPassword));
        };

        if camera.kind != greyhack_behaviour::DeviceKind::Camera {
            return Ok(Err(CctvFailure::WrongPassword));
        }

        let Some(password) = password else {
            return Ok(Ok(None));
        };
        let users = self.users(computer).await?;
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

        if let Some(router) = self.router_of(computer).await? {
            for device in self.devices(router.id).await? {
                for npc in self.npcs_on(device).await? {
                    let name = self
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
                    .smart(device)
                    .await?
                    .is_some_and(|smart| smart.is_broken(self.time_seconds))
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
