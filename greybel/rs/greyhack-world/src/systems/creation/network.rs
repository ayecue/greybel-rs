//! Network creation: lazy creation on first address access, explicit options, and the shared
//! hub, forward, and content-seed helpers.

use greyhack_behaviour::{DeviceKind, NetworkKind, is_lan_ip, is_valid_unreserved_ip};

use crate::generation::{
    ComputerOptions, NetworkOptions, computer, kernel_router, network, random_content,
};
use crate::{
    ComputerDraft, ComputerId, Contact, ForwardLayout, Gender, LanPlace, NetworkDraft, NetworkId,
    Port, Result, Rng, RouterDraft, RouterId, Word, World,
};

/// Database name for hubs, which are not computers in Grey Hack (`HubNet`).
const HUB_NAME: &str = "hub";

impl World {
    /// Returns an existing public router or generates its network on first access
    /// (`ServerMap.GetRouter`) for valid non-reserved addresses. Invalid, reserved, and LAN
    /// addresses return `None`.
    ///
    /// # Errors
    ///
    /// Returns any database lookup or network-generation error.
    pub async fn router_or_generate(&mut self, public_ip: &str) -> Result<Option<RouterId>> {
        if !is_valid_unreserved_ip(public_ip) {
            return Ok(None);
        }

        if let Some(router) = self.router_at(public_ip).await? {
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

    /// Creates a router with configured or generated computers.
    ///
    /// # Errors
    ///
    /// Returns any database error from address allocation, domain allocation, or network insertion.
    pub async fn add_network(&mut self, options: &NetworkOptions) -> Result<RouterId> {
        let mut rng = self.rng.fork("network");
        let public_ip = match &options.public_ip {
            Some(address) => address.clone(),
            None => loop {
                let address = rng.public_ip();

                if self.router_at(&address).await?.is_none() {
                    break address;
                }
            },
        };
        let essid = options
            .essid
            .clone()
            .unwrap_or_else(|| network::wifi_name(&mut rng));
        let domain = match &options.domain {
            Some(domain) => domain.clone(),
            None => greyhack_behaviour::network_domain(&self.world_seed, &public_ip),
        };
        let surname = Word::Surname.generate(&mut rng);
        let first_name = Gender::pick(&mut rng).first_name(&mut rng);
        let contact = Contact {
            mail: format!("{surname}@{domain}"),
            name: format!("{first_name} {surname}"),
            phone: rng.phone(),
        };
        let contact_password = Word::Password.generate(&mut rng);

        self.add_mail_account(&contact.mail, &contact_password)
            .await?;

        let kernel_router = kernel_router(&mut rng);
        let position = options
            .position
            .unwrap_or_else(|| greyhack_behaviour::position_for_ip(&public_ip));
        let kind = greyhack_behaviour::kind_for_ip(&self.world_seed, &public_ip);
        let (local_ip, plan) = self.lan_plan(&public_ip, kind, options.computers.is_none());
        let network_id = self
            .db
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
        let root_password = Word::Password.generate(&mut Rng::root_password(self.seed, &public_ip));
        let router = self
            .insert_router(
                &RouterDraft {
                    essid: Some(essid),
                    bssid: Some(rng.mac()),
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
                            self.add_router_log(&mut computer, self.mission_router_log(mission));
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
            self.finish_mission(network_id, &public_ip, kind, plan, mission)
                .await?;
        }

        Ok(router)
    }

    /// Generates the device of `device_kind` at `local_ip` in the network of `kind` at `public_ip`
    /// from its address-derived generator, with the random files of `Router.GenerateRandomContent`
    /// when `with_random_files`. Also returns the router forwards of its service ports.
    pub(crate) fn generate_computer(
        &self,
        (public_ip, local_ip): (&str, &str),
        options: &ComputerOptions,
        device_kind: DeviceKind,
        kind: NetworkKind,
        with_random_files: bool,
    ) -> (ComputerDraft, Vec<Port>) {
        let mut rng = Rng::stable(self.seed, &[public_ip, local_ip]);
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

    /// Adds a hub at `place` in `network`; hubs are rows without hardware or files.
    pub(super) async fn add_hub(
        &mut self,
        network: NetworkId,
        place: &LanPlace,
    ) -> Result<ComputerId> {
        self.db
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

    /// Forwards each of `forwards` on `router` to its port on the computer with the LAN address in
    /// `addresses`; forwards to addresses without a computer or port are skipped.
    pub(super) async fn insert_forwards(
        &mut self,
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
                self.db
                    .insert_port_forward(router, forward.number, computer, forward.port)
                    .await?;
            }
        }

        Ok(())
    }

    /// Returns the seed of `Router.GenerateRandomContent` for the computer at `lan_ip` in the
    /// network at `public_ip`. The game derives it from the network seed and the LAN address; a
    /// stable generator of both reproduces it for administrator restoration.
    pub(crate) fn content_seed(&self, public_ip: &str, lan_ip: &str) -> i32 {
        Rng::stable(self.seed, &[public_ip, lan_ip, "random content"]).next_u64() as i32
    }
}
