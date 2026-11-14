//! Network construction from an explicit layout, including copied Grey Hack networks.

use crate::{
    ComputerId, DeviceLayout, ForwardLayout, LanPlace, LayoutFailure, NetworkDraft,
    NetworkGeneration, NetworkLayout, Result, RouterId, WorldError,
};

/// Explicit-layout construction.
impl NetworkGeneration {
    pub async fn add_network_layout(
        &self,
        layout: &NetworkLayout,
    ) -> Result<(RouterId, Option<ComputerId>)> {
        let network = self
            .networks
            .insert_network(&NetworkDraft {
                kind: layout.kind,
                has_plan: false,
                public_ip: layout.public_ip.clone(),
                domain: layout.domain.clone(),
                contact: None,
                position: greyhack_behaviour::position_for_ip(&layout.public_ip),
            })
            .await?;

        if let Some(admin) = &layout.admin {
            self.npcs.set_network_admin(network, None, admin).await?;
        }

        if layout.disabled {
            self.networks.disable_network(network, true).await?;
        }

        self.networks
            .set_net_speed(network, layout.net_speed)
            .await?;

        let mut nodes: Vec<ComputerId> = Vec::with_capacity(layout.nodes.len());
        let mut main = None;
        let mut player = None;
        let mut addresses: Vec<(&str, ComputerId)> = Vec::new();
        let mut tables: Vec<(RouterId, &[ForwardLayout])> = Vec::new();

        for (index, node) in layout.nodes.iter().enumerate() {
            let parent = match node.parent {
                Some(parent) if parent < index => Some(nodes[parent]),
                Some(parent) => {
                    return Err(WorldError::Layout(LayoutFailure::ParentOrder {
                        index,
                        parent,
                    }));
                }
                None => None,
            };
            let id = match &node.device {
                DeviceLayout::Router {
                    draft,
                    ports,
                    forwards,
                } => {
                    let place = parent.map(LanPlace::device_under);
                    let router = self
                        .libraries
                        .insert_full_router(draft, network, place.as_ref())
                        .await?;

                    for port in ports {
                        let library = self
                            .files
                            .library_row(port.service.library_kind(), &port.version)
                            .await?;

                        self.computers
                            .insert_port(router.computer(), port, library)
                            .await?;
                    }

                    main.get_or_insert(router);
                    tables.push((router, forwards));
                    addresses.push((&draft.computer.local_ip, router.computer()));

                    router.computer()
                }
                DeviceLayout::Hub => {
                    let Some(parent) = parent else {
                        return Err(WorldError::Layout(LayoutFailure::HubWithoutParent {
                            index,
                        }));
                    };

                    self.add_hub(network, &LanPlace::device_under(parent))
                        .await?
                }
                DeviceLayout::Computer(draft) => {
                    let (Some(parent), Some(main)) = (parent, main) else {
                        return Err(WorldError::Layout(LayoutFailure::ComputerWithoutParent {
                            index,
                        }));
                    };
                    let place = LanPlace::computer_under(parent, main, &node.gateway);
                    let computer = self
                        .libraries
                        .insert_full_computer(draft, network, &place)
                        .await?;

                    if let Some(smart) = &draft.smart {
                        self.appliances.save_smart(computer, Some(smart)).await?;
                    }

                    if draft.is_player {
                        player = Some(computer);
                    }

                    addresses.push((&draft.local_ip, computer));

                    computer
                }
            };

            nodes.push(id);
        }

        let Some(main) =
            main.filter(|_| matches!(layout.nodes[0].device, DeviceLayout::Router { .. }))
        else {
            return Err(WorldError::Layout(LayoutFailure::FirstNodeNotRouter));
        };

        for (router, forwards) in tables {
            self.insert_forwards(router, forwards, &addresses).await?;
        }

        if layout.rental_admin {
            self.jobs
                .spawn_rental_admin(&layout.public_ip, network)
                .await?;
        }

        Ok((main, player))
    }
}
