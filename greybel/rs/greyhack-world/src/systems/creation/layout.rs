//! Network construction from an explicit layout, including copied Grey Hack networks.

use crate::{
    ComputerId, DeviceLayout, ForwardLayout, LanPlace, LayoutFailure, NetworkDraft, NetworkLayout,
    Result, RouterId, World, WorldError,
};

impl World {
    /// Adds the network described by `layout` node by node. Returns its main router and optional
    /// player computer. Computers contain only draft-provided state; no services are installed.
    ///
    /// # Errors
    ///
    /// Returns [`WorldError::Layout`] for missing or forward parent references, computers without
    /// parents, or a non-router first node; otherwise returns database errors.
    pub(crate) async fn add_network_layout(
        &mut self,
        layout: &NetworkLayout,
    ) -> Result<(RouterId, Option<ComputerId>)> {
        let network = self
            .db
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
            self.db.set_network_admin(network, None, admin).await?;
        }

        if layout.disabled {
            self.db.disable_network(network, true).await?;
        }

        self.db.set_net_speed(network, layout.net_speed).await?;

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
                        .db
                        .insert_router(draft, network, place.as_ref())
                        .await?;

                    for port in ports {
                        let library = self
                            .db
                            .library_row(port.service.library_kind(), &port.version)
                            .await?;

                        self.db
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
                    let computer = self.db.insert_computer(draft, network, &place).await?;

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
            self.spawn_rental_admin(&layout.public_ip, network).await?;
        }

        Ok((main, player))
    }
}
