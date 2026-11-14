//! Network creation: lazy creation on first address access, explicit options, and the shared
//! hub, forward, and content-seed helpers.

use greyhack_behaviour::{
    DeviceKind, NetworkKind, is_lan_ip, is_rumor_time, is_valid_unreserved_ip,
};

use crate::generation::{
    ComputerOptions, NetworkOptions, computer, kernel_router, network, random_content,
};
use crate::{
    ComputerDraft, ComputerId, Contact, ForwardLayout, Gender, LanPlace, NetworkDraft, NetworkId,
    Port, Result, Rng, RouterDraft, RouterId, Word,
};

/// Database name for hubs, which are not computers in Grey Hack (`HubNet`).
const HUB_NAME: &str = "hub";

/// Network lookup, creation, and shared generation helpers.
impl NetworkGeneration {
    pub async fn router_or_generate(&self, public_ip: &str) -> Result<Option<RouterId>> {
        if !is_valid_unreserved_ip(public_ip) {
            return Ok(None);
        }

        if let Some(router) = self.networks.routers.router_at(public_ip).await? {
            return Ok(Some(router.id));
        }

        if is_lan_ip(public_ip) {
            return Ok(None);
        }

        Ok(Some(
            self.add_network(&NetworkOptions {
                public_ip: Some(public_ip.to_owned()),
                ..NetworkOptions::default()
            })
            .await?,
        ))
    }

    pub async fn add_network(&self, options: &NetworkOptions) -> Result<RouterId> {
        let mut rng = self.world.world_mut().rng.fork("network");
        let public_ip = match &options.public_ip {
            Some(address) => address.clone(),
            None => loop {
                let address = rng.public_ip();

                if self.networks.routers.router_at(&address).await?.is_none() {
                    break address;
                }
            },
        };
        let (bssid, generated_essid) = network::wifi(&self.world.world_seed(), &[&public_ip]);
        let essid = options.essid.clone().unwrap_or(generated_essid);
        let domain = match &options.domain {
            Some(domain) => domain.clone(),
            None => greyhack_behaviour::network_domain(&self.world.world_seed(), &public_ip),
        };
        let surname = Word::Surname.generate(&mut rng);
        let first_name = Gender::pick(&mut rng).first_name(&mut rng);
        let contact = Contact {
            mail: format!("{surname}@{domain}"),
            name: format!("{first_name} {surname}"),
            phone: rng.phone(),
        };
        let contact_password = Word::Password.generate(&mut rng);

        self.mail
            .add_mail_account(&contact.mail, &contact_password)
            .await?;

        let kernel_router = kernel_router(&mut rng);
        let position = options
            .position
            .unwrap_or_else(|| greyhack_behaviour::position_for_ip(&public_ip));
        let kind = greyhack_behaviour::kind_for_ip(&self.world.world_seed(), &public_ip);
        let (local_ip, plan) = self.lan_plan(
            &public_ip,
            kind,
            options.computers.is_none(),
            is_rumor_time(self.world.time_seconds()),
        );
        let network_id = self
            .networks
            .insert_network(&NetworkDraft {
                kind,
                has_plan: plan.is_some(),
                public_ip: public_ip.clone(),
                domain: domain.clone(),
                contact: Some(contact),
                position,
            })
            .await?;
        // All routers and switches in a network share the main router root password (`LanRouter`).
        let root_password =
            Word::Password.generate(&mut Rng::root_password(self.world.seed(), &public_ip));
        let router = self
            .insert_router(
                &RouterDraft {
                    essid: Some(essid),
                    bssid: Some(bssid),
                    wifi_password: options
                        .wifi_password
                        .clone()
                        .unwrap_or_else(|| Word::Password.generate(&mut rng)),
                    firewall_rules: plan
                        .as_ref()
                        .map(|plan| plan.devices[0].firewall.clone())
                        .unwrap_or_default(),
                    firewall_active: true,
                    computer: {
                        let mut computer = self.router_computer(
                            &public_ip,
                            DeviceKind::Router,
                            &local_ip,
                            &root_password,
                            *kernel_router,
                        );

                        if let Some(mission) = plan.as_ref().and_then(|plan| plan.mission.as_ref())
                        {
                            self.mission_placement.add_router_log(
                                &mut computer,
                                self.mission_placement.mission_router_log(mission),
                            );
                        }

                        computer
                    },
                },
                network_id,
                None,
            )
            .await?;

        let network_computers = match &plan {
            Some(plan) => {
                self.build_lan(router, &public_ip, &root_password, plan)
                    .await?
            }
            None => {
                self.add_computers(
                    router,
                    kind,
                    options.computers.unwrap_or_default(),
                    &mut rng,
                )
                .await?
            }
        };

        self.configure_record_service(kind, &network_computers, &domain, &mut rng)
            .await?;
        self.add_network_npcs(&network_computers, &mut rng).await?;
        self.add_random_mails(&network_computers, &public_ip, kind)
            .await?;

        if let Some(plan) = &plan
            && let Some(mission) = &plan.mission
        {
            self.mission_placement
                .finish_mission(network_id, &public_ip, plan, mission)
                .await?;
        }

        Ok(router)
    }

    pub fn generate_computer(
        &self,
        (public_ip, local_ip): (&str, &str),
        options: &ComputerOptions,
        device_kind: DeviceKind,
        kind: NetworkKind,
        with_random_files: bool,
    ) -> (ComputerDraft, Vec<Port>) {
        let mut rng = Rng::stable(self.world.seed(), &[public_ip, local_ip]);
        let (mut draft, port_forwards) =
            computer::computer(&mut rng, local_ip, options, device_kind);

        if with_random_files {
            random_content::add_to_draft(
                &mut draft,
                self.content_seed(public_ip, local_ip),
                kind == NetworkKind::Neurobox,
            );
        }

        (draft, port_forwards)
    }

    pub async fn add_hub(&self, network: NetworkId, place: &LanPlace) -> Result<ComputerId> {
        self.computers
            .insert_computer_row(
                &ComputerDraft {
                    kind: DeviceKind::Hub,
                    ..ComputerDraft::new(HUB_NAME, "")
                },
                network,
                Some(place),
            )
            .await
    }

    pub async fn insert_forwards(
        &self,
        router: RouterId,
        forwards: &[ForwardLayout],
        addresses: &[(&str, ComputerId)],
    ) -> Result<()> {
        for forward in forwards {
            let target = addresses
                .iter()
                .find(|(address, _)| *address == forward.lan_ip)
                .map(|(_, computer)| *computer);

            if let Some(computer) = target {
                self.networks
                    .insert_port_forward(router, forward.number, computer, forward.port)
                    .await?;
            }
        }

        Ok(())
    }

    pub fn content_seed(&self, public_ip: &str, lan_ip: &str) -> i32 {
        Rng::stable(self.world.seed(), &[public_ip, lan_ip, "random content"]).next_u64() as i32
    }
}

/// A world service with only its declared dependencies.
pub struct NetworkGeneration {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystems>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) installs: greyhack_world_core::Dep<greyhack_world_services::Installs>,
    pub(crate) jobs: greyhack_world_core::Dep<greyhack_world_npcs::JobService>,
    pub(crate) mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
    pub(crate) npcs: greyhack_world_core::Dep<greyhack_world_npcs::NpcService>,
    pub(crate) libraries: greyhack_world_core::Dep<greyhack_world_libraries::LibraryService>,
    pub(crate) updates: greyhack_world_core::Dep<greyhack_world_libraries::Updates>,
    pub(crate) traffic: greyhack_world_core::Dep<greyhack_world_smart_devices::Traffic>,
    pub(crate) appliances: greyhack_world_core::Dep<greyhack_world_smart_devices::Appliances>,
    pub(crate) mission_placement: greyhack_world_core::Dep<crate::MissionPlacement>,
}

impl NetworkGeneration {
    pub(crate) fn new(registry: &std::rc::Rc<greyhack_world_modules::Registry>) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
            installs: registry.get(),
            jobs: registry.get(),
            mail: registry.get(),
            npcs: registry.get(),
            libraries: registry.get(),
            updates: registry.get(),
            traffic: registry.get(),
            appliances: registry.get(),
            mission_placement: registry.get(),
        }
    }
}
