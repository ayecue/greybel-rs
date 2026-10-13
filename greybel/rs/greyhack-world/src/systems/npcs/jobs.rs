//! NPC roles, death, firing, and hiring (`NetworkLan`, `NpcInfo`, `NpcHiring`).

use greyhack_behaviour::{DEATH_DELAY_SECONDS, NetworkKind, assign_role, draw_plate};

use crate::{
    CompanyAlign, ComputerId, Gender, JobRole, NetworkAdmin, NetworkId, Npc, NpcSchedule, PersonId,
    Result, Rng, Word, WorkPerformance, World,
};

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

impl World {
    /// Assigns roles and cars to newly added `npcs` in order (`NpcInfo.NpcInfo`). Draws a network
    /// role with at most one project manager (`NetworkLan.GlobalLan.AssignRandomJob`) and gives
    /// half a plate derived from network address and name
    /// (`LicensePlateGenerator.GenerateLicensePlate`). Uses an isolated generator to preserve other
    /// network content.
    pub(in crate::systems) async fn assign_jobs(
        &mut self,
        network: NetworkId,
        npcs: &[(PersonId, ComputerId, String)],
    ) -> Result<()> {
        let Some(row) = self.db.network(network).await? else {
            return Ok(());
        };
        let public_ip = self.network_public_ip(network).await?;
        let mut rng = Rng::stable(self.seed, &[&public_ip, "jobs"]);
        let mut roles = JobRole::available_for(row.kind == NetworkKind::Police).to_vec();

        for (person, _, user) in npcs {
            let Some(mut npc) = self.db.npc(*person).await? else {
                continue;
            };

            npc.job_role = assign_role(&mut roles, &mut rng);
            npc.license_plate = draw_plate(&self.world_seed, &mut rng, &public_ip, user);
            self.db.save_npc_job(&npc).await?;
        }

        Ok(())
    }

    /// Assigns job `role` and optional `license_plate` to NPC `npc` for sandbox use.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_npc_job(
        &mut self,
        npc: PersonId,
        role: JobRole,
        license_plate: Option<String>,
    ) -> Result<()> {
        let Some(mut row) = self.db.npc(npc).await? else {
            return Ok(());
        };

        row.job_role = role;
        row.license_plate = license_plate;

        self.db.save_npc_job(&row).await
    }

    /// Sets job performance `work_performance` of `npc` (`NpcInfo.workPerformance`) as copied from
    /// a Grey Hack network.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_npc_work_performance(
        &mut self,
        npc: PersonId,
        work_performance: WorkPerformance,
    ) -> Result<()> {
        let Some(mut row) = self.db.npc(npc).await? else {
            return Ok(());
        };

        row.work_performance = work_performance;

        self.db.save_npc_job(&row).await
    }

    /// Sets the company stance of `npc` and the mail address of its zero-day contact
    /// (`NpcInfo.SetCompanyAlign`, `NpcInfo.SetDayZeroContact`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_npc_company_align(
        &mut self,
        npc: PersonId,
        company_align: CompanyAlign,
        day_zero_contact: Option<String>,
    ) -> Result<()> {
        let Some(mut row) = self.db.npc(npc).await? else {
            return Ok(());
        };

        row.company_align = company_align;
        row.day_zero_contact = day_zero_contact;

        self.db.save_npc_job(&row).await
    }

    /// Kills `npc` now (`NpcInfo.SetDead`) and returns whether this opened the position.
    pub(crate) async fn set_dead(&mut self, npc: &Npc) -> Result<bool> {
        self.set_unavailable(npc, false, Some(self.time_seconds))
            .await
    }

    /// Schedules `npc` death five minutes ahead (`NpcInfo.ScheduleDeath`) and returns whether this
    /// opened the position.
    pub(crate) async fn schedule_death(&mut self, npc: &Npc) -> Result<bool> {
        self.set_unavailable(npc, false, Some(self.time_seconds + DEATH_DELAY_SECONDS))
            .await
    }

    /// Fires `npc` without killing them (`NpcInfo.SetFired`) and returns whether this opened the
    /// position.
    pub(crate) async fn fire(&mut self, npc: &Npc) -> Result<bool> {
        self.set_unavailable(npc, true, None).await
    }

    /// Opens `npc`'s position with `dead_at` for dying or fired NPCs unless already open
    /// (`NpcInfo.SetUnavailable`). Dead or fired NPCs log out and never return
    /// (`Persona.NextCita`).
    async fn set_unavailable(
        &mut self,
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
        self.db.save_npc_job(&npc).await?;

        if let Some(computer) = self.db.computer(npc.computer).await? {
            self.db
                .open_job(computer.network, npc.job_role, npc.computer)
                .await?;

            // `NpcInfo.SetUnavailable`: the company's market data is synced, and a missing
            // administrator costs the owning guild ranking points.
            if self.is_listed_on_stock_market(computer.network).await? {
                self.crown_ranking_dirty = true;
            }

            if npc.job_role == JobRole::Admin {
                self.apply_admin_absence_penalty(computer.network).await?;
            }
        }

        self.reschedule_npc_switch(npc.person, dead_at.unwrap_or(self.time_seconds))
            .await?;

        Ok(true)
    }

    /// Fires NPC `user` in the network at `public_ip` (`NetworkLan.FireNpc`). Returns `None` if no
    /// network exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn fire_npc(
        &mut self,
        public_ip: &str,
        user: &str,
    ) -> Result<Option<Result<(), FireFailure>>> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(None);
        };
        let Some(npc) = self.npc_in_network(router.network, user).await? else {
            return Ok(Some(Err(FireFailure::UnknownPerson)));
        };

        if npc.is_unavailable_for_work() {
            return Ok(Some(Err(FireFailure::Unavailable)));
        }

        if npc.is_hired_recently(self.time_seconds) {
            return Ok(Some(Err(FireFailure::HiredRecently)));
        }

        self.fire(&npc).await?;

        Ok(Some(Ok(())))
    }

    /// Hires an NPC for `role` into the first open position in network `public_ip`
    /// (`BankHelperServer.HireNpcServerRpc`, `NetworkLan.HireNpc`). Returns `None` without a
    /// network. Uses Grey Hack's first role candidate (`NpcHiring.GetNpcs`), creates an account,
    /// opposite shift when occupied by a living NPC, mail and bank accounts, smartphone, and a car
    /// for half. Hired administrators administer the network.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn hire_npc(
        &mut self,
        public_ip: &str,
        role: JobRole,
    ) -> Result<Option<Result<PersonId, HireFailure>>> {
        if role.code() > JobRole::Admin.code() {
            return Ok(Some(Err(HireFailure::InvalidJob)));
        }

        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(None);
        };
        let Some(computer) = self.db.close_job(router.network, role).await? else {
            return Ok(Some(Err(HireFailure::NoVacancy(role))));
        };
        let hire_time = self.time_seconds;
        let mut rng = Rng::stable(
            self.seed,
            &[public_ip, role.game_name(), &hire_time.to_string()],
        );
        let colleagues = self.db.npcs_on(computer).await?;
        let mut user = Word::Surname.generate(&mut rng);

        while self.db.user(computer, &user).await?.is_some() {
            user = Word::Surname.generate(&mut rng);
        }

        let password = Word::Password.generate(&mut rng);

        if let Err(failure) = self.add_user(computer, &user, &password).await? {
            unreachable!("a new account of an unused name: {failure:?}");
        }

        let name = format!("{} {user}", Gender::pick(&mut rng).first_name(&mut rng));
        let Some(person) = self.add_npc(computer, &user, &name, hire_time).await? else {
            unreachable!("the account was just added");
        };
        let Some(mut npc) = self.db.npc(person).await? else {
            unreachable!("the NPC was just added");
        };
        // Candidates start on the day shift. Candidates sharing a living colleague's shift use the
        // opposite shift and draw its work schedule (`NetworkLan.HireNpc`).
        let day_shift = !colleagues
            .iter()
            .any(|colleague| !colleague.is_dead(hire_time) && colleague.schedule.is_day_shift());

        npc.job_role = role;
        npc.license_plate = draw_plate(&self.world_seed, &mut rng, public_ip, &user);
        npc.hire_time = Some(hire_time);
        npc.schedule = NpcSchedule::new(&mut rng, day_shift);
        self.db.save_npc_job(&npc).await?;
        self.add_npc_phone(computer, person, &user, &mut rng)
            .await?;

        if role == JobRole::Admin {
            let admin = NetworkAdmin::draw(&mut rng);

            npc.work_performance = admin.work_performance;
            self.db.save_npc_job(&npc).await?;
            self.db
                .set_network_admin(router.network, Some(person), &admin)
                .await?;
            self.mark_company_admin_arrival(router.network).await?;
        }

        if self.is_listed_on_stock_market(router.network).await? {
            self.crown_ranking_dirty = true;
        }

        Ok(Some(Ok(person)))
    }
}

#[cfg(test)]
#[path = "tests/jobs.rs"]
pub(in crate::systems) mod tests;
