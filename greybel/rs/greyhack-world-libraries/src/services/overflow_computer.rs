//! Overflow attacks against computer libraries: requirements, rewards, and logs.

use greyhack_behaviour::{
    CredentialError, DeviceKind, GUEST_USER, LIBRARY_FOLDER, NetworkKind, Requirement, Reward,
    compare_versions, is_device, is_valid_ip, library_path, validate_credential,
};
use greyhack_world_core::{ComputerId, Result, RouterId};

use super::report::{
    AttackResult, AttackReward, ExploitLog, ExploitOutcome, ExploitReport, UnmetRequirement,
    attack, report, reward_report,
};
use crate::Weakness;

/// Methods of [`World`] for exploits overflow computer.
impl OverflowService {
    pub async fn overflow_file(
        &self,
        computer: ComputerId,
        library_path: &str,
        address: &str,
        unsec_value: &str,
        args: &str,
    ) -> Result<ExploitReport> {
        if self.networks.settings.is_disabled(computer).await? {
            return Ok(report(ExploitLog::Nothing, ExploitOutcome::DisabledNetwork));
        }

        // Grey Hack validates the containing folder before vulnerability lookup
        // (`MetaLibIntrinsics.overflow`).
        if library_path.rsplit_once('/').map(|(parent, _)| parent) != Some(LIBRARY_FOLDER) {
            return Ok(report(ExploitLog::OutsideLib, ExploitOutcome::Nothing));
        }

        let weakness = self
            .generator
            .full_file_library(computer, library_path, true)
            .await?
            .and_then(|library| library.weakness(address, unsec_value).cloned());
        let Some(weakness) = weakness else {
            return Ok(report(ExploitLog::MissingWeakness, ExploitOutcome::Nothing));
        };

        if self.guilds.is_crown_shield_network(computer).await? {
            return Ok(report(ExploitLog::CrownShield, ExploitOutcome::Nothing));
        }

        let mut library = None;

        if let Some(unmet) = self
            .unmet_requirement(computer, &weakness, &mut library)
            .await?
        {
            return Ok(attack(
                library,
                AttackResult::Unmet(unmet),
                ExploitOutcome::Nothing,
            ));
        }

        let report = self.perform(computer, &weakness, args, library).await?;

        self.versions
            .count_attack_use(
                greyhack_behaviour::path::file_name(library_path),
                (address, unsec_value, weakness.reward),
                &report.outcome,
            )
            .await?;

        Ok(report)
    }

    pub async fn perform(
        &self,
        computer: ComputerId,
        weakness: &Weakness,
        args: &str,
        library: Option<String>,
    ) -> Result<ExploitReport> {
        // `NpcInfo.IsSuperAdmin`, `NetworkLan.IsSuperAdminRental`: target-network super
        // administrators force guest on every computer except the player's.
        let super_admin = match self.computers.computer(computer).await? {
            Some(row) => self.jobs.has_super_admin(row.network).await?,
            None => false,
        };
        let player = self.world.is_player(computer);
        let mut intercepted = false;
        let (reward, outcome) = if weakness.reward == Reward::Computer {
            intercepted = super_admin && !player;

            self.computer_reward(computer, weakness, args, super_admin)
                .await?
        } else if let Some(user) = self.shell_user(computer, weakness).await? {
            intercepted = super_admin && !player;

            let user = if intercepted {
                GUEST_USER.to_owned()
            } else {
                user
            };

            match weakness.reward {
                Reward::FirewallDisable => self.disable_firewall(computer).await?,
                Reward::OverrideSettings => self.override_settings(computer).await?,
                Reward::TrafficLightControl => self.traffic_light_control(computer).await?,
                Reward::Shell => (
                    AttackReward::Privileges(user.clone()),
                    ExploitOutcome::Shell { computer, user },
                ),
                Reward::RandomFolder => {
                    let folder = weakness.random_path.clone();

                    match self.files.lookup.folder(computer, &folder, true).await? {
                        None => (
                            AttackReward::MissingResource(folder),
                            ExploitOutcome::Nothing,
                        ),
                        Some(folder_node) => (
                            AttackReward::Privileges(user.clone()),
                            ExploitOutcome::File {
                                computer,
                                node: folder_node.id,
                                path: folder,
                                user,
                            },
                        ),
                    }
                }
                Reward::ChangePass => self.change_pass(computer, user, args).await?,
                Reward::Computer => unreachable!("handled above"),
            }
        } else {
            // All rewards except computer resolve the user first and fail if absent.
            (AttackReward::NoUser, ExploitOutcome::Nothing)
        };

        Ok(reward_report(library, intercepted, reward, outcome))
    }

    pub async fn log_overflow_shell(
        &self,
        library_name: &str,
        prev_ip: &str,
        origin: ComputerId,
        computer: ComputerId,
    ) -> Result<String> {
        let port = self
            .computers
            .processes
            .library_service_port(computer, library_name)
            .await?;

        self.computers
            .logs
            .write_bounce_logs(prev_ip, port, origin, computer)
            .await?;

        self.networks.topology.public_ip(origin).await
    }
}

impl OverflowService {
    pub async fn unmet_requirement(
        &self,
        computer: ComputerId,
        weakness: &Weakness,
        library: &mut Option<String>,
    ) -> Result<Option<UnmetRequirement>> {
        let Some(target) = self.networks.topology.computer(computer).await? else {
            return Ok(Some(UnmetRequirement::TargetMissing));
        };
        let id = target.id;

        for requirement in &weakness.requirements {
            match requirement {
                Requirement::Library => {
                    let name = weakness
                        .required_lib
                        .map(|kind| kind.file_name())
                        .unwrap_or_default();
                    let library_version = self
                        .generator
                        .full_file_library(id, &library_path(&name), true)
                        .await?
                        .map(|library| library.version);

                    match library_version {
                        None => return Ok(Some(UnmetRequirement::MissingLibrary(name))),
                        Some(current) => {
                            *library = Some(name.clone());

                            if compare_versions(&current, &weakness.req_lib_version).is_lt() {
                                return Ok(Some(UnmetRequirement::OldLibrary {
                                    library: name,
                                    minimum: weakness.req_lib_version.clone(),
                                    current,
                                }));
                            }
                        }
                    }
                }
                Requirement::NumberUsersRegister => {
                    if u32::try_from(self.computers.users.users(id).await?.len())
                        .expect("value fits the target integer type")
                        < weakness.num_register_users
                    {
                        return Ok(Some(UnmetRequirement::TooFewUsers));
                    }
                }
                Requirement::PortForward => {
                    if self.networks.routers.port_forwards_to(id).await?
                        < weakness.num_port_forward as usize
                    {
                        return Ok(Some(UnmetRequirement::TooFewPortForwards));
                    }
                }
                Requirement::ConnGateway => {
                    if self.networks.routers.gateway_connections(id).await?
                        < weakness.num_conn_gateway as usize
                    {
                        return Ok(Some(UnmetRequirement::TooFewGatewayConnections));
                    }
                }
                Requirement::LocalNetwork => return Ok(Some(UnmetRequirement::RouterOnly)),
                Requirement::AnyActiveUser => {
                    if !self.computers.users.is_any_active_user(id).await? {
                        return Ok(Some(UnmetRequirement::NoActiveUser));
                    }
                }
                Requirement::RootActiveUser => {
                    if !self.computers.users.is_root_active_user(id).await? {
                        return Ok(Some(UnmetRequirement::NoActiveRoot));
                    }
                }
                Requirement::GuestActiveUser => {
                    if !self.computers.users.is_guest_active(id).await? {
                        return Ok(Some(UnmetRequirement::NoGuest));
                    }
                }
                Requirement::PathExist => {
                    if self
                        .files
                        .lookup
                        .entry(id, &weakness.path_exist, true)
                        .await?
                        .is_none()
                    {
                        return Ok(Some(UnmetRequirement::MissingPath(
                            weakness.path_exist.clone(),
                        )));
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn computer_reward(
        &self,
        computer: ComputerId,
        weakness: &Weakness,
        args: &str,
        guest: bool,
    ) -> Result<(AttackReward, ExploitOutcome)> {
        let mut target = computer;

        if let Some(router) = self.networks.routers.router(RouterId::of(computer)).await? {
            if args.is_empty() || !is_valid_ip(args) {
                return Ok((AttackReward::InvalidLanAddress, ExploitOutcome::Failure));
            }

            let lookup = self
                .networks
                .lan
                .lan(router.network)
                .await?
                .and_then(|lan| {
                    lan.find(args).map(|id| {
                        let node = lan.node(id);

                        (node.key, is_device(node.kind) || !node.hidden)
                    })
                });

            if let Some((device, _)) = lookup {
                target = device;
            }

            if !lookup.is_some_and(|(_, visible)| visible) {
                return Ok((
                    AttackReward::MissingComputer(args.to_owned()),
                    ExploitOutcome::Failure,
                ));
            }
        }

        let Some(user) = self.shell_user(target, weakness).await? else {
            return Ok((AttackReward::NoUser, ExploitOutcome::Failure));
        };
        let user = if guest { GUEST_USER.to_owned() } else { user };

        Ok((
            AttackReward::Credentials(user.clone()),
            ExploitOutcome::Computer {
                computer: target,
                user,
            },
        ))
    }

    pub async fn override_settings(
        &self,
        computer: ComputerId,
    ) -> Result<(AttackReward, ExploitOutcome)> {
        let appliance = self
            .computers
            .computer(computer)
            .await?
            .is_some_and(|row| matches!(row.kind, DeviceKind::Fridge | DeviceKind::Microwave));
        let smart = if appliance {
            self.appliances.smart(computer).await?
        } else {
            None
        };
        let Some(mut smart) = smart else {
            return Ok((AttackReward::NotAppliance, ExploitOutcome::Failure));
        };

        if smart.override_enabled {
            return Ok((AttackReward::OverrideAlreadyOn, ExploitOutcome::Success));
        }

        smart.override_enabled = true;
        self.appliances.save_smart(computer, Some(&smart)).await?;

        Ok((AttackReward::OverrideOn, ExploitOutcome::Success))
    }

    pub async fn traffic_light_control(
        &self,
        computer: ComputerId,
    ) -> Result<(AttackReward, ExploitOutcome)> {
        // Use the computer's original network kind, not its current network
        // (`Computer.GetTipoRed`).
        let Some(row) = self
            .computers
            .computer(computer)
            .await?
            .filter(|row| row.network_kind == NetworkKind::Police)
        else {
            return Ok((AttackReward::NotPolice, ExploitOutcome::Failure));
        };

        // Use the main router of the current network (`Computer.GetMainRouter`).
        let mut traffic = self.traffic.traffic(row.network).await?;

        if !traffic.override_enabled {
            traffic.override_enabled = true;
            self.traffic.save_traffic(row.network, &traffic).await?;
        }

        Ok((AttackReward::TrafficOverride, ExploitOutcome::Success))
    }

    pub async fn disable_firewall(
        &self,
        computer: ComputerId,
    ) -> Result<(AttackReward, ExploitOutcome)> {
        let router = RouterId::of(computer);
        let Some(_) = self.networks.routers.router(router).await? else {
            return Ok((AttackReward::NotRouter, ExploitOutcome::Failure));
        };
        let mut rules = self.networks.firewall_rules(router).await?;

        self.networks.clear_firewall(router).await?;
        for rule in &mut rules {
            rule.allow = true;
            self.networks.insert_firewall_rule(router, rule).await?;
        }

        Ok((AttackReward::FirewallOff, ExploitOutcome::Success))
    }

    pub async fn change_pass(
        &self,
        computer: ComputerId,
        user: String,
        args: &str,
    ) -> Result<(AttackReward, ExploitOutcome)> {
        if user == GUEST_USER {
            return Ok((AttackReward::GuestPassword, ExploitOutcome::Failure));
        }

        match validate_credential(args) {
            Err(CredentialError::InvalidCharacters) => {
                return Ok((AttackReward::InvalidPassword(user), ExploitOutcome::Failure));
            }
            Err(CredentialError::TooLong) => {
                return Ok((AttackReward::LongPassword, ExploitOutcome::Failure));
            }
            Ok(()) => {}
        }

        self.computers
            .users
            .set_password(computer, &user, args)
            .await?;

        Ok((AttackReward::PasswordChange(user), ExploitOutcome::Success))
    }

    pub async fn shell_user(
        &self,
        computer: ComputerId,
        weakness: &Weakness,
    ) -> Result<Option<String>> {
        if weakness.user != "normal_user" {
            return Ok(Some(weakness.user.clone()));
        }

        self.computers.users.normal_user(computer).await
    }
}

/// A world service with only its declared dependencies.
pub struct OverflowService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
    pub(crate) jobs: greyhack_world_modules::Dep<greyhack_world_npcs::JobService>,
    pub(crate) appliances:
        greyhack_world_modules::Dep<greyhack_world_smart_devices::ApplianceService>,
    pub(crate) traffic: greyhack_world_modules::Dep<greyhack_world_smart_devices::Traffic>,
    pub(crate) generator: greyhack_world_modules::Dep<crate::Generator>,
    pub(crate) versions: greyhack_world_modules::Dep<crate::VersionService>,
}

impl OverflowService {
    pub(crate) fn new(registry: &std::rc::Rc<greyhack_world_modules::Registry>) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
            guilds: registry.get(),
            jobs: registry.get(),
            appliances: registry.get(),
            traffic: registry.get(),
            generator: registry.get(),
            versions: registry.get(),
        }
    }
}
