//! Generated LAN plans and their materialized devices and computers.

use greyhack_behaviour::{
    DeviceKind, FileData, NetworkKind, ROOT_USER, ServiceId, main_router_address, path,
    random_device_kind,
};
use mono_random::Random;

use crate::generation::lan::{self, ComputerPlan, LanPlan};
use crate::generation::missions::{MissionPlacement, request_for};
use crate::generation::random_content::RandomContent;
use crate::generation::{ComputerOptions, kernel_router, network};
use crate::{
    ComputerDraft, ComputerId, LanPlace, NetworkId, Result, Rng, RouterDraft, RouterId, Word, World,
};

impl World {
    /// Returns the main-router LAN address and, with `with_plan`, the generated LAN plan for
    /// `public_ip`. Uses an isolated generator for administrator restoration. The main router uses
    /// `192.168.0.1` or `192.168.1.1` from `Router`'s first draw.
    pub(crate) fn lan_plan(
        &self,
        public_ip: &str,
        kind: NetworkKind,
        with_plan: bool,
    ) -> (String, Option<LanPlan>) {
        let mut rng = Rng::stable(self.seed, &[public_ip, "lan"]);
        let local_ip = main_router_address(rng.range(0, 1));
        let mission = (with_plan && Self::takes_missions(kind))
            .then(|| request_for(&self.world_seed, public_ip))
            .flatten();
        let plan = with_plan.then(|| lan::plan(&mut rng, kind, &local_ip, mission));

        (local_ip, plan)
    }

    /// Adds planned devices and computers below main `router`, installs main-router forwards, and
    /// returns standard computers and servers excluding cameras and appliances.
    pub(super) async fn build_lan(
        &mut self,
        router: RouterId,
        public_ip: &str,
        root_password: &str,
        plan: &LanPlan,
    ) -> Result<Vec<ComputerId>> {
        let Some(main) = self.db.router(router).await? else {
            return Ok(Vec::new());
        };
        let mut devices = vec![router.computer()];

        for device in plan.devices.iter().skip(1) {
            let parent = devices[device.parent.unwrap_or(0)];
            let place = LanPlace::device_under(parent);
            let id = if device.kind == DeviceKind::Hub {
                self.add_hub(main.network, &place).await?
            } else {
                let mut rng = Rng::stable(self.seed, &[public_ip, &device.local_ip]);
                let (bssid, essid) = device.wifi.clone().unzip();
                let wifi_password = Word::Password.generate(&mut rng);
                let kernel = kernel_router(&mut rng);
                let computer = self.router_computer(
                    public_ip,
                    device.kind,
                    &device.local_ip,
                    root_password,
                    *kernel,
                );
                let lan_router = self
                    .insert_router(
                        &RouterDraft {
                            essid,
                            bssid,
                            wifi_password,
                            firewall_rules: device.firewall.clone(),
                            firewall_active: true,
                            computer,
                        },
                        main.network,
                        Some(&place),
                    )
                    .await?;

                lan_router.computer()
            };

            devices.push(id);
        }

        let mut addresses: Vec<(&str, ComputerId)> = Vec::new();
        let mut computers = Vec::new();

        for (index, device) in plan.devices.iter().enumerate() {
            for computer_plan in &device.computers {
                let place =
                    LanPlace::computer_under(devices[index], router, &computer_plan.gateway);
                let computer = self
                    .add_computer_of_plan(
                        main.network,
                        public_ip,
                        computer_plan,
                        &place,
                        plan.mission.as_ref(),
                    )
                    .await?;

                addresses.push((&computer_plan.local_ip, computer));

                if computer_plan.kind == DeviceKind::Computer {
                    computers.push(computer);
                }
            }
        }

        self.insert_forwards(router, &plan.forwards, &addresses)
            .await?;

        Ok(computers)
    }

    /// Adds `computer_plan` at `place` using its address-derived generator and exactly the planned
    /// services and ports.
    async fn add_computer_of_plan(
        &mut self,
        network: NetworkId,
        public_ip: &str,
        computer_plan: &ComputerPlan,
        place: &LanPlace,
        mission: Option<&MissionPlacement>,
    ) -> Result<ComputerId> {
        let kind = self.network(network).await?.map(|network| network.kind);
        let draft = self.draft_of_plan(
            public_ip,
            computer_plan,
            kind.unwrap_or(NetworkKind::Unknown),
            mission,
        );
        let computer = self.db.insert_computer(&draft, network, place).await?;

        for port in &draft.ports {
            self.install_service(computer, port.service, &port.version)
                .await?;
        }

        if let Some(kind) = kind {
            self.configure_shop(computer, kind).await?;
        }

        Ok(computer)
    }

    /// Builds `computer_plan` of a network of `kind` from its address-derived generator with
    /// exactly the planned services, and its random files (`Router.GenerateRandomContent`).
    pub(super) fn draft_of_plan(
        &self,
        public_ip: &str,
        computer_plan: &ComputerPlan,
        kind: NetworkKind,
        mission: Option<&MissionPlacement>,
    ) -> ComputerDraft {
        let options = ComputerOptions {
            services: Some(
                computer_plan
                    .services
                    .iter()
                    .map(|service| service.service.name().to_owned())
                    .collect(),
            ),
            ..ComputerOptions::default()
        };
        let mission =
            mission.filter(|mission| mission.assignment_at(&computer_plan.local_ip).is_some());
        // A computer with a mission has no random content, except in Neurobox networks (`Router.GeneraLanPc`).
        let (mut draft, _) = self.generate_computer(
            (public_ip, &computer_plan.local_ip),
            &options,
            computer_plan.kind,
            kind,
            computer_plan.kind == DeviceKind::Computer
                && (mission.is_none() || kind == NetworkKind::Neurobox),
        );

        for port in &mut draft.ports {
            if let Some(service) = computer_plan
                .services
                .iter()
                .find(|service| service.port == port.number)
            {
                port.closed = service.closed;
            }
        }

        if let Some(mission) = mission {
            self.apply_mission_to_draft(public_ip, computer_plan, mission, &mut draft);
        }

        draft
    }

    /// Adds `count` computers to the main-router subnet for a fixed-size network. The first runs
    /// services identifying network `kind`.
    pub(super) async fn add_computers(
        &mut self,
        router: RouterId,
        kind: NetworkKind,
        count: u32,
        rng: &mut Rng,
    ) -> Result<Vec<ComputerId>> {
        let mut network_computers = Vec::new();
        let mut cameras = 0;
        let mut fridge = false;
        let mut microwave = false;

        for index in 0..count {
            let device_kind = if index == 0 {
                DeviceKind::Computer
            } else {
                random_device_kind(rng, kind, cameras, fridge, microwave)
            };
            let options = ComputerOptions::for_computer(kind, index == 0);

            match device_kind {
                DeviceKind::Camera => cameras += 1,
                DeviceKind::Fridge => fridge = true,
                DeviceKind::Microwave => microwave = true,
                _ => {}
            }

            let computer = self.add_device_to(router, &options, device_kind).await?;

            if device_kind == DeviceKind::Computer {
                network_computers.extend(computer);
            }
        }

        Ok(network_computers)
    }

    /// Adds the conversations of `Router.GenerateRandomContent` to the mailboxes of NPCs on
    /// `computers` of the network at `public_ip`. The files of the same pass are part of each
    /// computer's draft; the mails need the NPCs of the network, which exist only now.
    pub(super) async fn add_random_mails(
        &mut self,
        computers: &[ComputerId],
        public_ip: &str,
        kind: NetworkKind,
    ) -> Result<()> {
        for &computer in computers {
            let Some(row) = self.db.computer(computer).await? else {
                continue;
            };
            let mut users = vec![ROOT_USER.to_owned()];

            users.extend(
                self.db
                    .users(computer)
                    .await?
                    .into_iter()
                    .map(|user| user.name)
                    .filter(|user| user != ROOT_USER),
            );

            let npcs = self.db.npcs_on(computer).await?;
            let mut random = Random::new(self.content_seed(public_ip, &row.local_ip));
            let content = RandomContent::draw(
                &mut random,
                &users,
                kind == NetworkKind::Neurobox,
                &|_, _| false,
                &|user| npcs.iter().any(|npc| npc.user == user),
            );

            for mail in content.mails {
                let Some(npc) = npcs.iter().find(|npc| npc.user == mail.user) else {
                    continue;
                };
                let Some(address) = self
                    .db
                    .person(npc.person)
                    .await?
                    .and_then(|person| person.mail_address)
                else {
                    continue;
                };

                self.add_conversation_mail(&address, &mail.conversation, &mail.receive_user)
                    .await?;
            }
        }

        Ok(())
    }

    /// Creates a police/student database and adds its launcher to every non-root user home
    /// (`Router.PostGenComputer`).
    pub(super) async fn configure_record_service(
        &mut self,
        kind: NetworkKind,
        computers: &[ComputerId],
        domain: &str,
        rng: &mut Rng,
    ) -> Result<()> {
        let service = match kind {
            NetworkKind::Police => ServiceId::Criminals,
            NetworkKind::University => ServiceId::Students,
            _ => return Ok(()),
        };
        let viewer = service.record_viewer().expect("a SQL service has a viewer");
        let mut server = None;

        for &computer in computers {
            if self.has_service(computer, service).await? {
                server = Some(computer);
                break;
            }
        }

        let Some(server) = server else {
            return Ok(());
        };

        if let Some(records) = network::records(rng, service, domain) {
            self.write_file(
                server,
                service.config_path(),
                FileData::text(&records.to_json()),
            )
            .await?;
        }

        let target = self.service_address(server, service).await?;

        for &computer in computers {
            for user in self.users(computer).await? {
                let home = path::home(&user.name);

                self.write_launcher(computer, &format!("{home}/{viewer}"), &target)
                    .await?;
            }
        }

        Ok(())
    }
}
