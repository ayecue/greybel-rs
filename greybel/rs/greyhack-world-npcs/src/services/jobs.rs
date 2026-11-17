//! NPC roles, death, firing, and hiring (`NetworkLan`, `NpcInfo`, `NpcHiring`).

use greyhack_behaviour::{
    CompanyAlign, DEATH_DELAY_SECONDS, Gender, JobRole, NetworkAdmin, NetworkKind, NpcSchedule,
    Rng, Word, WorkPerformance, assign_role, draw_plate,
};
use greyhack_world_core::{ComputerId, NetworkId, PersonId, Result};

use crate::{NetworkCreation, Npc, NpcJobChange, NpcSwitch};

/// NPC firing errors (`NetworkLan.FireNpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireFailure {
    /// No NPC with that name belongs to the network.
    UnknownPerson,
    /// The position is already open because the NPC is dead, dying, or fired
    /// (`NpcInfo.IsUnavailableForWork`).
    Unavailable,
    /// The NPC was hired less than one month ago.
    HiredRecently,
}

/// NPC hiring errors (`BankHelperServer.HireNpcServerRpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HireFailure {
    /// Police roles cannot be hired.
    InvalidJob,
    /// The network company has no open position for the role.
    NoVacancy(JobRole),
}

/// Methods of [`World`] for npcs jobs.
impl JobService {
    pub async fn assign_jobs(
        &self,
        network: NetworkId,
        npcs: &[(PersonId, ComputerId, String)],
    ) -> Result<()> {
        let Some(row) = self.networks.network(network).await? else {
            return Ok(());
        };
        let public_ip = self.networks.topology.network_public_ip(network).await?;
        let mut rng = Rng::stable(self.world.seed(), &[&public_ip, "jobs"]);
        let mut roles = JobRole::available_for(row.kind == NetworkKind::Police).to_vec();

        for (person, _, user) in npcs {
            let Some(mut npc) = self.npcs.npc(*person).await? else {
                continue;
            };

            npc.job_role = assign_role(&mut roles, &mut rng);
            npc.license_plate = draw_plate(&self.world.world_seed(), &mut rng, &public_ip, user);
            self.npcs.save_npc_job(&npc).await?;
        }

        Ok(())
    }

    pub async fn set_npc_job(
        &self,
        npc: PersonId,
        role: JobRole,
        license_plate: Option<String>,
    ) -> Result<()> {
        let Some(mut row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };

        row.job_role = role;
        row.license_plate = license_plate;

        self.npcs.save_npc_job(&row).await
    }

    pub async fn set_npc_work_performance(
        &self,
        npc: PersonId,
        work_performance: WorkPerformance,
    ) -> Result<()> {
        let Some(mut row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };

        row.work_performance = work_performance;

        self.npcs.save_npc_job(&row).await
    }

    pub async fn set_npc_company_align(
        &self,
        npc: PersonId,
        company_align: CompanyAlign,
        day_zero_contact: Option<String>,
        zero_day_index: Option<u32>,
    ) -> Result<()> {
        let Some(mut row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };

        row.company_align = company_align;
        row.day_zero_contact = day_zero_contact;
        row.zero_day_index = zero_day_index;

        self.npcs.save_npc_job(&row).await
    }

    /// Marks the NPC as the source of a first-rumor chat (`NpcInfo.SetRumor`).
    pub async fn set_npc_rumor(&self, npc: PersonId) -> Result<()> {
        let Some(mut row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };

        row.rumor = true;
        self.npcs.save_npc_job(&row).await
    }

    pub async fn set_dead(&self, npc: &Npc) -> Result<bool> {
        self.set_unavailable(npc, false, Some(self.world.time_seconds()))
            .await
    }

    pub async fn schedule_death(&self, npc: &Npc) -> Result<bool> {
        self.set_unavailable(
            npc,
            false,
            Some(self.world.time_seconds() + DEATH_DELAY_SECONDS),
        )
        .await
    }

    pub async fn fire(&self, npc: &Npc) -> Result<bool> {
        self.set_unavailable(npc, true, None).await
    }

    pub async fn fire_npc(
        &self,
        public_ip: &str,
        user: &str,
    ) -> Result<Option<Result<(), FireFailure>>> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(None);
        };
        let Some(npc) = self.npcs.npc_in_network(router.network, user).await? else {
            return Ok(Some(Err(FireFailure::UnknownPerson)));
        };

        if npc.is_unavailable_for_work() {
            return Ok(Some(Err(FireFailure::Unavailable)));
        }

        if npc.is_hired_recently(self.world.time_seconds()) {
            return Ok(Some(Err(FireFailure::HiredRecently)));
        }

        self.fire(&npc).await?;

        Ok(Some(Ok(())))
    }

    pub async fn hire_npc(
        &self,
        public_ip: &str,
        role: JobRole,
    ) -> Result<Option<Result<PersonId, HireFailure>>> {
        if role.code() > JobRole::Admin.code() {
            return Ok(Some(Err(HireFailure::InvalidJob)));
        }

        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(None);
        };
        let Some(computer) = self.npcs.npcs.close_job(router.network, role).await? else {
            return Ok(Some(Err(HireFailure::NoVacancy(role))));
        };
        let hire_time = self.world.time_seconds();
        let mut rng = Rng::stable(
            self.world.seed(),
            &[public_ip, role.game_name(), &hire_time.to_string()],
        );
        let colleagues = self.npcs.npcs_on(computer).await?;
        let mut user = Word::Surname.generate(&mut rng);

        while self.computers.user(computer, &user).await?.is_some() {
            user = Word::Surname.generate(&mut rng);
        }

        let password = Word::Password.generate(&mut rng);

        if let Err(failure) = self
            .computers
            .users
            .add_user(computer, &user, &password)
            .await?
        {
            unreachable!("a new account of an unused name: {failure:?}");
        }

        let name = format!("{} {user}", Gender::pick(&mut rng).first_name(&mut rng));
        let Some(person) = self.npcs.add_npc(computer, &user, &name, hire_time).await? else {
            unreachable!("the account was just added");
        };
        let Some(mut npc) = self.npcs.npc(person).await? else {
            unreachable!("the NPC was just added");
        };
        // Candidates start on the day shift. Candidates sharing a living colleague's shift use the
        // opposite shift and draw its work schedule (`NetworkLan.HireNpc`).
        let day_shift = !colleagues
            .iter()
            .any(|colleague| !colleague.is_dead(hire_time) && colleague.schedule.is_day_shift());

        npc.job_role = role;
        npc.license_plate = draw_plate(&self.world.world_seed(), &mut rng, public_ip, &user);
        npc.hire_time = Some(hire_time);
        npc.schedule = NpcSchedule::new(&mut rng, day_shift);
        self.npcs.save_npc_job(&npc).await?;
        self.npcs
            .add_npc_phone(computer, person, &user, &mut rng)
            .await?;

        if role == JobRole::Admin {
            let admin = NetworkAdmin::draw(&mut rng);

            npc.work_performance = admin.work_performance;
            self.npcs.save_npc_job(&npc).await?;
            self.npcs
                .set_network_admin(router.network, Some(person), &admin)
                .await?;
            self.world.events().publish(crate::AdminArrival {
                network: router.network,
            });
        }

        self.world.events().publish(NpcJobChange {
            network: router.network,
        });

        Ok(Some(Ok(person)))
    }
}

impl JobService {
    pub async fn set_unavailable(
        &self,
        npc: &Npc,
        fired: bool,
        dead_at: Option<u64>,
    ) -> Result<bool> {
        if npc.is_unavailable_for_work() {
            return Ok(false);
        }

        let mut npc = npc.clone();

        npc.fired = fired;
        npc.dead_at = dead_at;
        self.npcs.save_npc_job(&npc).await?;

        if let Some(computer) = self.computers.computer(npc.computer).await? {
            self.npcs
                .npcs
                .open_job(computer.network, npc.job_role, npc.computer)
                .await?;

            // `NpcInfo.SetUnavailable`: the company's market data is synced, and a missing
            // administrator costs the owning guild ranking points.
            self.world.events().publish(NpcJobChange {
                network: computer.network,
            });

            if npc.job_role == JobRole::Admin {
                self.world.events().publish(crate::AdminAbsence {
                    network: computer.network,
                });
            }
        }

        NpcSwitch::reschedule_with(
            &self.world,
            &self.scheduler,
            npc.person,
            dead_at.unwrap_or(self.world.time_seconds()),
        )
        .await?;

        Ok(true)
    }
}

/// NPC employment, hiring, firing, and rented-server administrators.
pub struct JobService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) npcs: greyhack_world_modules::Dep<crate::NpcService>,
    pub(crate) bank: greyhack_world_modules::Dep<greyhack_world_economy::Bank>,
    pub(crate) network_creation: greyhack_world_modules::Dep<dyn NetworkCreation>,
    pub(crate) scheduler: std::rc::Rc<greyhack_world_effects::Scheduler>,
}

impl JobService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
        scheduler: std::rc::Rc<greyhack_world_effects::Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            computers: registry.get(),
            networks: registry.get(),
            npcs: registry.get(),
            bank: registry.get(),
            network_creation: registry.get(),
            scheduler,
        }
    }
}
