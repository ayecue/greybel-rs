//! Sniffers along a network path: starting them and finding which see a connection
//! (`NetworkLan.GetSniffers`).

use greyhack_behaviour::{DeviceKind, is_lan_ip};

use crate::model::{ComputerId, SnifferPacket};
use crate::world::{WorkId, WorkResult};
use crate::{Result, RouterId, World};

impl World {
    /// Starts a sniffer on `computer` for the next packet through its device or LAN. Returns its
    /// work ID, or `None` if the computer is absent.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn start_sniffer(&mut self, computer: ComputerId) -> Result<Option<WorkId>> {
        let Some(computer) = self.db.computer(computer).await? else {
            return Ok(None);
        };

        Ok(Some(
            self.db.insert_sniffer(computer.id, computer.router).await?,
        ))
    }

    /// Delivers `packet` to selected sniffers and ends them as `NetworkLan.GetSniffers` consumes
    /// captures.
    pub(super) async fn capture(
        &mut self,
        sniffers: &[WorkId],
        packet: &SnifferPacket,
    ) -> Result<()> {
        for &id in sniffers {
            self.db.remove_sniffer(id).await?;
            self.work.push_end(id, WorkResult::Capture(packet.clone()));
        }

        Ok(())
    }

    /// Restricts sniffers to exactly `devices`, excluding other machines attached to their parents.
    pub(super) async fn sniffers_on_path(&self, devices: &[ComputerId]) -> Result<Vec<WorkId>> {
        let mut parents = Vec::new();
        let mut roots = Vec::new();

        for &device in devices {
            match self.db.computer(device).await?.and_then(|row| row.router) {
                Some(parent) => parents.push(parent),
                None => roots.push(device),
            }
        }

        let mut sniffers = Vec::new();

        for id in self.db.sniffers_on(&parents).await? {
            if self
                .db
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
            sniffers.extend(self.db.active_sniffers_on(root).await?);
        }

        sniffers.sort_unstable();
        sniffers.dedup();

        Ok(sniffers)
    }

    /// Returns `device` and each ancestor through the main router.
    async fn ancestry(&self, device: ComputerId) -> Result<Vec<ComputerId>> {
        let mut path = Vec::new();
        let mut current = Some(device);

        while let Some(device) = current {
            if path.contains(&device) {
                break;
            }

            path.push(device);
            current = self
                .db
                .computer(device)
                .await?
                .and_then(|row| row.router)
                .map(RouterId::computer);
        }

        Ok(path)
    }

    /// Adds sibling devices exposed by source or destination hubs (`NetworkLan.GetSniffers`).
    async fn add_hub_siblings(
        &self,
        endpoint: ComputerId,
        path: &mut Vec<ComputerId>,
    ) -> Result<()> {
        let Some(row) = self.db.computer(endpoint).await? else {
            return Ok(());
        };
        let Some(parent) = row.parent else {
            return Ok(());
        };

        if self
            .db
            .computer(parent)
            .await?
            .is_none_or(|row| row.kind != DeviceKind::Hub)
        {
            return Ok(());
        }

        for peer in self.db.computers_in(row.network).await? {
            if peer.parent == Some(parent) && peer.id != endpoint && !path.contains(&peer.id) {
                path.push(peer.id);
            }
        }

        Ok(())
    }

    /// Returns devices whose sniffers observe a login from `source` to `destination` at `address`.
    pub(super) async fn login_path(
        &self,
        source: ComputerId,
        destination: ComputerId,
        address: &str,
    ) -> Result<Vec<ComputerId>> {
        let source_path = self.ancestry(source).await?;
        let lan = is_lan_ip(address);
        let mut path_destination = destination;

        // Connections to a router LAN address stop at that router even if its ports forward
        // elsewhere.
        if lan
            && let Some(root) = source_path.last()
            && self
                .db
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
