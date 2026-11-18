//! Network construction from an explicit layout, including copied Grey Hack networks.

use greyhack_world_core::{
    ComputerId, LanPlace, LayoutFailure, NetworkDraft, Result, RouterId, WorldError,
};

use crate::{DeviceLayout, ForwardLayout, NetworkGeneration, NetworkLayout};

/// Mutable indexes and generated entities of an explicit layout.
#[derive(Default)]
struct LayoutBuilder<'a> {
    nodes: Vec<ComputerId>,
    main: Option<RouterId>,
    player: Option<ComputerId>,
    addresses: Vec<(&'a str, ComputerId)>,
    tables: Vec<(RouterId, &'a [ForwardLayout])>,
}

impl<'a> LayoutBuilder<'a> {
    async fn add_node(
        &mut self,
        generation: &NetworkGeneration,
        network: greyhack_world_core::NetworkId,
        index: usize,
        node: &'a crate::LanNodeLayout,
    ) -> Result<ComputerId> {
        let parent = match node.parent {
            Some(parent) if parent < index => Some(self.nodes[parent]),
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
                let mut draft_with_choices = draft.clone();

                generation
                    .updates
                    .apply_network_choices_to_layout(network, &mut draft_with_choices.computer)
                    .await?;

                let router = generation
                    .libraries
                    .insert_full_router(&draft_with_choices, network, place.as_ref())
                    .await?;

                for port in ports {
                    let library = generation
                        .files
                        .library_row(port.service.library_kind(), &port.version)
                        .await?;

                    generation
                        .computers
                        .insert_port(router.computer(), port, library)
                        .await?;
                }

                self.main.get_or_insert(router);
                self.tables.push((router, forwards));
                self.addresses
                    .push((&draft.computer.local_ip, router.computer()));

                router.computer()
            }
            DeviceLayout::Hub => {
                let Some(parent) = parent else {
                    return Err(WorldError::Layout(LayoutFailure::HubWithoutParent {
                        index,
                    }));
                };

                generation
                    .add_hub(network, &LanPlace::device_under(parent))
                    .await?
            }
            DeviceLayout::Computer(draft) => {
                let (Some(parent), Some(main)) = (parent, self.main) else {
                    return Err(WorldError::Layout(LayoutFailure::ComputerWithoutParent {
                        index,
                    }));
                };
                let place = LanPlace::computer_under(parent, main, &node.gateway);
                let mut draft_with_choices = draft.clone();

                generation
                    .updates
                    .apply_network_choices_to_layout(network, &mut draft_with_choices)
                    .await?;

                let computer = generation
                    .libraries
                    .insert_full_computer(&draft_with_choices, network, &place)
                    .await?;

                if let Some(smart) = &draft.smart {
                    generation
                        .appliances
                        .save_smart(computer, Some(smart))
                        .await?;
                }

                if draft.is_player {
                    self.player = Some(computer);
                }

                self.addresses.push((&draft.local_ip, computer));

                computer
            }
        };

        Ok(id)
    }
}

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
                contact: layout.contact.clone(),
                position: layout.position,
            })
            .await?;

        if layout.library_choices.is_empty() {
            self.updates
                .ensure_network_library_choices(network, &layout.public_ip)
                .await?;
        } else {
            self.updates
                .set_network_library_choices(network, &layout.library_choices)
                .await?;
        }

        if let Some(admin) = &layout.admin {
            self.npcs.set_network_admin(network, None, admin).await?;
        }

        if layout.disabled {
            self.networks.disable_network(network, true).await?;
        }

        self.networks
            .set_net_speed(network, layout.net_speed)
            .await?;

        let mut builder = LayoutBuilder::default();

        for (index, node) in layout.nodes.iter().enumerate() {
            let id = builder.add_node(self, network, index, node).await?;

            builder.nodes.push(id);
        }

        let Some(main) = builder
            .main
            .filter(|_| matches!(layout.nodes[0].device, DeviceLayout::Router { .. }))
        else {
            return Err(WorldError::Layout(LayoutFailure::FirstNodeNotRouter));
        };

        for (router, forwards) in builder.tables {
            self.insert_forwards(router, forwards, &builder.addresses)
                .await?;
        }

        if layout.rental_admin {
            self.jobs
                .spawn_rental_admin(&layout.public_ip, network)
                .await?;
        }

        Ok((main, builder.player))
    }
}
