//! Router and switch computer creation and persistence.

use greyhack_behaviour::{DeviceKind, Rng, ServiceId};
use greyhack_world_core::{LanPlace, NetworkId, Result, RouterId};
use greyhack_world_libraries::{ComputerDraft, Library, RouterDraft};

use crate::NetworkGeneration;
use crate::generation::{computer, library_version};

/// Router and switch creation.
impl NetworkGeneration {
    pub fn router_computer(
        &self,
        public_ip: &str,
        kind: DeviceKind,
        local_ip: &str,
        root_password: &str,
        kernel: Library,
    ) -> Box<ComputerDraft> {
        let mut rng = Rng::stable(self.world.seed(), &[public_ip, local_ip, "router"]);
        let http_version = library_version(&mut rng);

        Box::new(computer::router_computer(
            &mut rng,
            kind,
            local_ip,
            root_password,
            kernel,
            &http_version,
        ))
    }

    pub async fn insert_router(
        &self,
        draft: &RouterDraft,
        network: NetworkId,
        place: Option<&LanPlace>,
    ) -> Result<RouterId> {
        let mut draft = draft.clone();

        self.updates
            .start_with_reported_libraries(network, &mut draft.computer)
            .await?;

        let router = self
            .libraries
            .insert_full_router(&draft, network, place)
            .await?;

        if let Some(port) = draft
            .computer
            .ports
            .iter()
            .find(|port| port.service == ServiceId::Http)
        {
            self.installs
                .install_service(router.computer(), ServiceId::Http, &port.version)
                .await?;
        }

        Ok(router)
    }
}
