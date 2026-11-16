//! Generated LAN plans and their materialized devices and computers.

use greyhack_behaviour::{
    DeviceDraw, DeviceKind, FileData, NetworkKind, ROOT_USER, Rng, ServiceId, Word,
    main_router_address, path,
};
use greyhack_world_core::{ComputerId, LanPlace, NetworkId, Result, RouterId};
use greyhack_world_libraries::{ComputerDraft, RouterDraft};
use greyhack_world_missions::{MissionPlacement, request_for};
use mono_random::Random;

use crate::generation::lan::{self, ComputerPlan, LanPlan};
use crate::generation::random_content::RandomContent;
use crate::generation::{ComputerOptions, kernel_router, network};
use crate::{MissionPlacement as MissionService, NetworkGeneration};

/// LAN planning and materialization.
impl NetworkGeneration {
    pub fn lan_plan(
        &self,
        public_ip: &str,
        kind: NetworkKind,
        with_plan: bool,
        zero_day_open: bool,
    ) -> (String, Option<LanPlan>) {
        let mut rng = Rng::stable(self.world.seed(), &[public_ip, "lan"]);
        let local_ip = main_router_address(rng.range(0, 1));
        let mission = (with_plan && MissionService::takes_missions(kind))
            .then(|| request_for(&self.world.world_seed(), public_ip, zero_day_open))
            .flatten();
        let plan = with_plan.then(|| {
            lan::plan(
                &mut rng,
                &self.world.world_seed(),
                public_ip,
                kind,
                &local_ip,
                mission,
            )
        });

        (local_ip, plan)
    }

    pub async fn build_lan(
        &self,
        router: RouterId,
        public_ip: &str,
        root_password: &str,
        plan: &LanPlan,
    ) -> Result<Vec<ComputerId>> {
        let Some(main) = self.networks.router(router).await? else {
            return Ok(Vec::new());
        };
        let mut devices = vec![router.computer()];

        for device in plan.devices.iter().skip(1) {
            let parent = devices[device.parent.unwrap_or(0)];
            let place = LanPlace::device_under(parent);
            let id = if device.kind == DeviceKind::Hub {
                self.add_hub(main.network, &place).await?
            } else {
                let mut rng = Rng::stable(self.world.seed(), &[public_ip, &device.local_ip]);
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
                        (plan.kind, plan.mission.as_ref()),
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

    pub fn draft_of_plan(
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
            self.mission_placement.apply_mission_to_draft(
                public_ip,
                computer_plan,
                mission,
                &mut draft,
            );
        }

        draft
    }

    pub async fn add_computers(
        &self,
        router: RouterId,
        kind: NetworkKind,
        count: u32,
        rng: &mut Rng,
    ) -> Result<Vec<ComputerId>> {
        let mut network_computers = Vec::new();
        let mut draw = DeviceDraw::default();

        for index in 0..count {
            let device_kind = if index == 0 {
                DeviceKind::Computer
            } else {
                draw.draw(rng, kind)
            };
            let options = ComputerOptions::for_computer(kind, index == 0);

            if device_kind == DeviceKind::Camera {
                draw.add_camera();
            }

            let computer = self.add_device_to(router, &options, device_kind).await?;

            if device_kind == DeviceKind::Computer {
                network_computers.extend(computer);
            }
        }

        Ok(network_computers)
    }

    pub async fn add_random_mails(
        &self,
        computers: &[ComputerId],
        public_ip: &str,
        kind: NetworkKind,
    ) -> Result<()> {
        for &computer in computers {
            let Some(row) = self.computers.computer(computer).await? else {
                continue;
            };
            let mut users = vec![ROOT_USER.to_owned()];

            users.extend(
                self.computers
                    .users(computer)
                    .await?
                    .into_iter()
                    .map(|user| user.name)
                    .filter(|user| user != ROOT_USER),
            );

            let npcs = self.npcs.npcs_on(computer).await?;
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
                let Some(address) = self.mail.mail_address(npc.person).await? else {
                    continue;
                };

                self.mail
                    .add_conversation_mail(&address, &mail.conversation, &mail.receive_user)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn configure_record_service(
        &self,
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
            if self
                .computers
                .processes
                .has_service(computer, service)
                .await?
            {
                server = Some(computer);
                break;
            }
        }

        let Some(server) = server else {
            return Ok(());
        };

        if let Some(records) = network::records(rng, service, domain) {
            self.files
                .changes
                .write_file(
                    server,
                    service.config_path(),
                    FileData::text(&records.to_json()),
                )
                .await?;
        }

        let target = self.installs.service_address(server, service).await?;

        for &computer in computers {
            for user in self.computers.users.users(computer).await? {
                let home = path::home(&user.name);

                self.installs
                    .write_launcher(computer, &format!("{home}/{viewer}"), &target)
                    .await?;
            }
        }

        Ok(())
    }
}

impl NetworkGeneration {
    pub async fn add_computer_of_plan(
        &self,
        network: NetworkId,
        public_ip: &str,
        computer_plan: &ComputerPlan,
        place: &LanPlace,
        (kind, mission): (NetworkKind, Option<&MissionPlacement>),
    ) -> Result<ComputerId> {
        let draft = self.draft_of_plan(public_ip, computer_plan, kind, mission);
        let computer = self
            .libraries
            .insert_full_computer(&draft, network, place)
            .await?;

        if let Some(smart) = &draft.smart {
            self.appliances.save_smart(computer, Some(smart)).await?;
        }

        for port in &draft.ports {
            self.installs
                .install_service(computer, port.service, &port.version)
                .await?;
        }

        // `Router.GeneraLanPc` runs `Router.PostGenComputer` before `Router.PostGenMission`.
        self.installs.configure_shop(computer, kind).await?;

        if let Some(placement) = mission
            && placement.assignment_at(&computer_plan.local_ip).is_some()
        {
            self.mission_placement
                .add_mission_files_over_services(
                    computer,
                    public_ip,
                    computer_plan,
                    placement,
                    &draft,
                )
                .await?;
        }

        Ok(computer)
    }
}
