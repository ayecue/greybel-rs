//! Computer and device creation in existing networks.

use greyhack_behaviour::{
    CLOCK_PROGRAM_NAME, CLOCK_PROGRAM_PATH, DeviceKind, FileData, NetworkKind,
};

use crate::generation::{ComputerOptions, computer};
use crate::{ComputerId, LanPlace, NetworkGeneration, Result, Rng, RouterId};

/// Computer and device creation.
impl NetworkGeneration {
    pub async fn add_computer_to(
        &self,
        router: RouterId,
        options: &ComputerOptions,
    ) -> Result<Option<ComputerId>> {
        self.add_device_to(router, options, DeviceKind::Computer)
            .await
    }

    pub async fn add_device_to(
        &self,
        router: RouterId,
        options: &ComputerOptions,
        device_kind: DeviceKind,
    ) -> Result<Option<ComputerId>> {
        // A device's content comes from generators of its stable addresses, so administrator
        // restoration reproduces original users, passwords, libraries, and services
        // (`RedGlobal.AdminRevertComputer`). The world generator still draws once per added device,
        // so that what it generates afterwards does not depend on where content comes from.
        self.world.world_mut().rng.skip(1);

        let Some(network) = self.networks.router(router).await? else {
            return Ok(None);
        };
        let used_addresses: Vec<String> = self
            .computers
            .computers_in(network.network)
            .await?
            .into_iter()
            .map(|computer| computer.local_ip)
            .collect();
        let mut lan_rng = Rng::stable(self.world.seed(), &[&network.public_ip, "lan"]);
        let Some(lan_ip) = computer::lan_address(
            &mut lan_rng,
            &network.local_ip,
            &used_addresses,
            options.lan_ip.as_ref(),
        ) else {
            return Ok(None);
        };
        let kind = self
            .networks
            .settings
            .network(network.network)
            .await?
            .map(|network| network.kind);
        let (mut draft, mut port_forwards) = self.generate_computer(
            (&network.public_ip, &lan_ip),
            options,
            device_kind,
            kind.unwrap_or(NetworkKind::Unknown),
            device_kind == DeviceKind::Computer,
        );

        self.updates
            .start_with_reported_libraries(network.network, &mut draft)
            .await?;

        for forward in &mut port_forwards {
            if let Some(port) = draft
                .ports
                .iter()
                .find(|port| port.number == forward.number)
            {
                forward.version.clone_from(&port.version);
            }
        }

        let computer = self
            .libraries
            .insert_full_computer(
                &draft,
                network.network,
                &LanPlace::computer_under(router.computer(), network.main, &network.local_ip),
            )
            .await?;

        if let Some(smart) = &draft.smart {
            self.appliances.save_smart(computer, Some(smart)).await?;
        }

        for port in &port_forwards {
            // All initial network forwards belong to the main router (`NetworkLan.PreAddServicio`).
            self.networks
                .insert_port_forward(network.main, port.number, computer, port.number)
                .await?;
            self.installs
                .install_service(computer, port.service, &port.version)
                .await?;
        }

        if device_kind == DeviceKind::Smartphone {
            self.add_clock_program(computer).await?;
        }

        if let Some(kind) = kind {
            self.installs.configure_shop(computer, kind).await?;
        }

        Ok(Some(computer))
    }

    pub async fn add_clock_program(&self, phone: ComputerId) -> Result<()> {
        self.files
            .changes
            .write_file(
                phone,
                CLOCK_PROGRAM_PATH,
                FileData::binary(FileData::binary_size(CLOCK_PROGRAM_NAME, false)),
            )
            .await?;

        Ok(())
    }
}
