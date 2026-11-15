//! NPC user operations.

use greyhack_behaviour::{
    ALPHANUMERIC, DeviceKind, FileData, Gender, NpcSchedule, Rng, Word, WorkPerformance,
    bank_config_path, hash_password, mail_config_path, npc_mail_address,
};
use greyhack_world_core::{ComputerId, NetworkId, Person, PersonId, Result};

use crate::{Npc, NpcService, NpcSwitch};

/// Methods of [`World`] for npcs people.
impl NpcService {
    pub async fn add_npc(
        &self,
        computer: ComputerId,
        user: &str,
        name: &str,
        next_action_at: u64,
    ) -> Result<Option<PersonId>> {
        self.add_npc_with_mail_name(computer, user, user, name, next_action_at)
            .await
    }

    pub async fn add_npc_with_mail_name(
        &self,
        computer: ComputerId,
        user: &str,
        mail_name: &str,
        name: &str,
        next_action_at: u64,
    ) -> Result<Option<PersonId>> {
        if self.computers.user(computer, user).await?.is_none() {
            return Ok(None);
        }

        let mut rng = self.world.world_mut().rng.fork("npc-accounts");
        let domain = self.mail.mail_domain(computer).await?;
        let mut attempt = 0;
        let mut mail_address = npc_mail_address(mail_name, &domain, attempt);

        while self.mail.mail_account(&mail_address).await?.is_some() {
            attempt += 1;
            mail_address = npc_mail_address(mail_name, &domain, attempt);
        }

        let mail_password = Word::Password.generate(&mut rng);

        self.mail
            .add_mail_account(&mail_address, &mail_password)
            .await?;

        let mut bank_number = Self::bank_account(&mut rng, &domain);

        while self.bank.bank_account(&bank_number).await?.is_some() {
            bank_number = Self::bank_account(&mut rng, &domain);
        }

        let bank_password = Word::Password.generate(&mut rng);

        self.bank
            .add_bank(&bank_number, &bank_password, 0.0)
            .await?;

        // `Persona.GeneraMail`/`SetBanco` store account:number and MD5-password pairs in Config
        // (`Persona.SetBanco`, `Computer.GeneraMails`).
        self.files
            .changes
            .write_file_as(
                computer,
                &mail_config_path(user),
                user,
                FileData::text(&format!("{mail_address}:{}", hash_password(&mail_password))),
            )
            .await?;
        self.files
            .changes
            .write_file_as(
                computer,
                &bank_config_path(user),
                user,
                FileData::text(&format!("{bank_number}:{}", hash_password(&bank_password))),
            )
            .await?;

        let id = self
            .computers
            .insert_person(&Person {
                id: PersonId(0),
                name: name.to_owned(),
                // NPC surnames match account names (`NetworkLan.AddNpcUser`).
                surname: user.to_owned(),
                gender: Gender::Male,
            })
            .await?;

        self.mail.set_mail_address(id, Some(&mail_address)).await?;
        self.bank.set_bank_number(id, Some(&bank_number)).await?;

        if let Some(mut account) = self.computers.user(computer, user).await? {
            account.person = Some(id);
            self.computers.save_user(computer, &account).await?;
        }

        // `NpcInfo.NpcInfo`: draws the new NPC work schedule for its assigned shift.
        let day_time = rng.range(0, 1) == 1;
        let schedule = NpcSchedule::new(&mut rng, day_time);
        // `NpcInfo.NpcInfo`: `workPerformance` is a uniform draw from `WorkStatus`, the last draw of
        // an NPC after its shift and schedule.
        let work_performance = *rng.pick(&WorkPerformance::ALL);

        self.npcs
            .insert_npc(id, computer, user, false, &schedule, work_performance)
            .await?;

        NpcSwitch::schedule_with(&self.world, &self.scheduler, next_action_at, id).await?;

        Ok(Some(id))
    }

    pub async fn set_person_mail_address(&self, person: PersonId, address: &str) -> Result<()> {
        let Some(old) = self.mail.mail_address(person).await? else {
            return Ok(());
        };

        self.mail.mail.rename_mail_account(&old, address).await
    }

    pub async fn appoint_network_admin(&self, npc: PersonId) -> Result<()> {
        let Some(row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };
        let Some(computer) = self.computers.computer(row.computer).await? else {
            return Ok(());
        };
        let Some(admin) = self.network_admins.network_admin(computer.network).await? else {
            return Ok(());
        };

        self.network_admins
            .set_network_admin(computer.network, Some(npc), &admin)
            .await
    }

    pub async fn is_network_admin(&self, id: PersonId) -> Result<bool> {
        let Some(npc) = self.npcs.npc(id).await? else {
            return Ok(false);
        };
        let Some(computer) = self.computers.computer(npc.computer).await? else {
            return Ok(false);
        };

        Ok(self
            .network_admins
            .network_admin_person(computer.network)
            .await?
            == Some(id))
    }

    pub async fn npc_in_network(&self, network: NetworkId, user: &str) -> Result<Option<Npc>> {
        Ok(self
            .npcs
            .npcs_in(network)
            .await?
            .into_iter()
            .find(|npc| npc.user == user))
    }

    pub async fn npc_on_computer(&self, computer: ComputerId, user: &str) -> Result<Option<Npc>> {
        Ok(self
            .npcs
            .npcs_on(computer)
            .await?
            .into_iter()
            .find(|npc| npc.user == user))
    }

    pub async fn has_smartphone(&self, network: NetworkId, person: PersonId) -> Result<bool> {
        Ok(self
            .computers
            .computers_in(network)
            .await?
            .into_iter()
            .any(|computer| {
                computer.kind == DeviceKind::Smartphone && computer.npc_owner == Some(person)
            }))
    }

    pub async fn npc(&self, id: PersonId) -> Result<Option<Npc>> {
        self.npcs.npc(id).await
    }

    pub async fn npcs_on(&self, computer: ComputerId) -> Result<Vec<Npc>> {
        self.npcs.npcs_on(computer).await
    }

    pub async fn add_npc_phone(
        &self,
        computer: ComputerId,
        npc: PersonId,
        user: &str,
        rng: &mut Rng,
    ) -> Result<()> {
        let password = Word::Password.generate(rng);
        let phone = self
            .network_creation
            .add_phone(computer, npc, user, &password)
            .await?;

        if let Some(phone) = phone {
            let path = mail_config_path(user);

            if let Some(content) = self.files.lookup.content(computer, &path, false).await? {
                self.files
                    .changes
                    .write_file_as(
                        phone,
                        &path,
                        user,
                        greyhack_behaviour::FileData::text(&content),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Generates an eight-character alphanumeric, domain-suffixed bank account ID
    /// (`OS.GeneraBankAccount`).
    fn bank_account(rng: &mut Rng, domain: &str) -> String {
        let id = rng.text(ALPHANUMERIC, 8);

        format!("{id}-{domain}")
    }
}
