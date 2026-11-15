//! Sniffers along a network path: starting them and finding which see a connection
//! (`NetworkLan.GetSniffers`).

use greyhack_behaviour::{DeviceKind, is_lan_ip};
use greyhack_world_core::{ComputerId, Result, RouterId};
use greyhack_world_effects::WorkId;

use crate::{SnifferPacket, SnifferService};

/// Methods of [`World`] for intrusion sniffer path.
impl SnifferService {
    pub async fn start_sniffer(&self, computer: ComputerId) -> Result<Option<WorkId>> {
        let Some(computer) = self.computers.computer(computer).await? else {
            return Ok(None);
        };

        Ok(Some(
            self.subscribers
                .insert_sniffer(computer.id, computer.router)
                .await?,
        ))
    }

    pub async fn capture(&self, sniffers: &[WorkId], packet: &SnifferPacket) -> Result<()> {
        for &id in sniffers {
            self.end(id, packet.clone()).await?;
        }

        Ok(())
    }

    pub async fn sniffers_on_path(&self, devices: &[ComputerId]) -> Result<Vec<WorkId>> {
        let mut parents = Vec::new();
        let mut roots = Vec::new();

        for &device in devices {
            match self
                .computers
                .computer(device)
                .await?
                .and_then(|row| row.router)
            {
                Some(parent) => parents.push(parent),
                None => roots.push(device),
            }
        }

        let mut sniffers = Vec::new();

        for id in self.subscribers.sniffers_on(&parents).await? {
            if self
                .subscribers
                .sniffer_computer(id)
                .await?
                .is_some_and(|computer| devices.contains(&computer))
            {
                sniffers.push(id);
            }
        }

        // Routers have no parent ID, so `sniffers_on` cannot find sniffers running directly on a
        // router.
        for root in roots {
            sniffers.extend(self.subscribers.active_sniffers_on(root).await?);
        }

        sniffers.sort_unstable();
        sniffers.dedup();

        Ok(sniffers)
    }

    pub async fn login_path(
        &self,
        source: ComputerId,
        destination: ComputerId,
        address: &str,
    ) -> Result<Vec<ComputerId>> {
        let source_path = self.ancestry(source).await?;
        let lan = is_lan_ip(address);
        let mut path_destination = destination;

        // ConnectionService to a router LAN address stop at that router even if its ports forward
        // elsewhere.
        if lan
            && let Some(root) = source_path.last()
            && self
                .computers
                .computer(*root)
                .await?
                .is_some_and(|row| row.local_ip == address)
        {
            path_destination = *root;
        }

        let destination_path = self.ancestry(path_destination).await?;
        let common = source_path.iter().find_map(|source| {
            destination_path
                .iter()
                .position(|device| device == source)
                .map(|destination| (*source, destination))
        });
        let mut path = Vec::new();

        if lan && let Some((common, destination_index)) = common {
            let source_index = source_path
                .iter()
                .position(|device| *device == common)
                .expect("the common ancestor came from the source path");

            path.extend_from_slice(&source_path[..=source_index]);
            path.extend(destination_path[..destination_index].iter().rev().copied());
        } else {
            // Public connections traverse the source LAN to its router, then the destination router
            // to the target.
            path.extend(source_path);
            path.extend(destination_path);
        }

        path.dedup();
        self.add_hub_siblings(source, &mut path).await?;
        self.add_hub_siblings(path_destination, &mut path).await?;

        Ok(path)
    }
}

impl SnifferService {
    pub async fn ancestry(&self, device: ComputerId) -> Result<Vec<ComputerId>> {
        let mut path = Vec::new();
        let mut current = Some(device);

        while let Some(device) = current {
            if path.contains(&device) {
                break;
            }

            path.push(device);
            current = self
                .computers
                .computer(device)
                .await?
                .and_then(|row| row.router)
                .map(RouterId::computer);
        }

        Ok(path)
    }

    pub async fn add_hub_siblings(
        &self,
        endpoint: ComputerId,
        path: &mut Vec<ComputerId>,
    ) -> Result<()> {
        let Some(row) = self.computers.computer(endpoint).await? else {
            return Ok(());
        };
        let Some(parent) = row.parent else {
            return Ok(());
        };

        if self
            .computers
            .computer(parent)
            .await?
            .is_none_or(|row| row.kind != DeviceKind::Hub)
        {
            return Ok(());
        }

        for peer in self.computers.computers_in(row.network).await? {
            if peer.parent == Some(parent) && peer.id != endpoint && !path.contains(&peer.id) {
                path.push(peer.id);
            }
        }

        Ok(())
    }
}
