//! Overflow attacks against router kernels.

use greyhack_behaviour::{LibraryKind, Requirement, Reward, is_lan_ip, same_subnet};

use super::report::{
    AttackResult, ExploitLog, ExploitOutcome, ExploitReport, UnmetRequirement, attack, report,
};
use crate::{ComputerId, LanFailure, OverflowService, Result, RouterId, Weakness};

/// Methods of [`World`] for exploits overflow router.
impl OverflowService {
    pub async fn overflow_router(
        &self,
        router: RouterId,
        address: &str,
        unsec_value: &str,
        session_computer: ComputerId,
        reach: (&str, ComputerId),
        args: &str,
    ) -> Result<ExploitReport> {
        if self
            .networks
            .settings
            .is_disabled(router.computer())
            .await?
        {
            return Ok(report(ExploitLog::Nothing, ExploitOutcome::DisabledNetwork));
        }

        let kernel = self.networks.routers.kernel_file(router).await?;
        let library = match &kernel {
            Some(kernel) => self.generator.full_library_of(kernel).await?,
            None => None,
        };
        let weakness = library.and_then(|library| library.weakness(address, unsec_value).cloned());
        let Some(weakness) = weakness else {
            return Ok(report(ExploitLog::MissingWeakness, ExploitOutcome::Nothing));
        };

        if self
            .guilds
            .is_crown_shield_network(router.computer())
            .await?
        {
            return Ok(report(ExploitLog::CrownShield, ExploitOutcome::Nothing));
        }

        if weakness.reward != Reward::FirewallDisable
            && let Some(failure) = self.router_reach(router, reach).await?
        {
            return Ok(report(
                ExploitLog::ConnectionRefusal(failure),
                ExploitOutcome::Nothing,
            ));
        }

        if let Some(unmet) = self
            .unmet_router_requirement(router, &weakness, session_computer)
            .await?
        {
            return Ok(attack(
                None,
                AttackResult::Unmet(unmet),
                ExploitOutcome::Nothing,
            ));
        }

        let report = self
            .perform(router.computer(), &weakness, args, None)
            .await?;

        self.versions
            .count_attack_use(
                &LibraryKind::KernelRouter.file_name(),
                (address, unsec_value, weakness.reward),
                &report.outcome,
            )
            .await?;

        Ok(report)
    }
}

impl OverflowService {
    pub async fn router_reach(
        &self,
        router: RouterId,
        (address, origin): (&str, ComputerId),
    ) -> Result<Option<LanFailure>> {
        let Some(origin_row) = self.computers.computer(origin).await? else {
            return Ok(None);
        };
        let Some(router_row) = self.networks.router(router).await? else {
            return Ok(None);
        };
        let source = if is_lan_ip(address) {
            origin_row.local_ip
        } else {
            self.networks.topology.public_ip(origin).await?
        };

        if source == address {
            return Ok(None);
        }

        self.networks
            .reach
            .firewall_ping_status(router_row.network, &source, address, 0)
            .await
    }

    pub async fn unmet_router_requirement(
        &self,
        router: RouterId,
        weakness: &Weakness,
        session_computer: ComputerId,
    ) -> Result<Option<UnmetRequirement>> {
        let devices = self.networks.topology.devices(router).await?.len();
        let too_few_users = weakness
            .requirements
            .contains(&Requirement::NumberUsersRegister)
            && (devices as u32) < weakness.num_register_users;

        if too_few_users {
            return Ok(Some(UnmetRequirement::TooFewUsers));
        }

        let Some(router) = self.networks.routers.router(router).await? else {
            return Ok(Some(UnmetRequirement::TargetMissing));
        };
        let local_ip = router.local_ip.clone();

        if weakness.requirements.contains(&Requirement::LocalNetwork)
            && !self
                .networks
                .topology
                .computer(session_computer)
                .await?
                .is_some_and(|computer| same_subnet(&computer.local_ip, &local_ip))
        {
            return Ok(Some(UnmetRequirement::OtherSubnet));
        }

        if weakness.requirements.contains(&Requirement::PortForward) {
            let forward_count = self
                .networks
                .router_config
                .port_forwards(router.id)
                .await?
                .into_iter()
                .filter(|forward| forward.lan_ip == local_ip)
                .count();

            if forward_count < weakness.num_port_forward as usize {
                return Ok(Some(UnmetRequirement::TooFewPortForwards));
            }
        }

        if weakness.requirements.contains(&Requirement::ConnGateway)
            && self
                .networks
                .routers
                .router_gateway_connections(router.id)
                .await?
                < weakness.num_conn_gateway as usize
        {
            return Ok(Some(UnmetRequirement::TooFewGatewayConnections));
        }

        Ok(None)
    }
}
