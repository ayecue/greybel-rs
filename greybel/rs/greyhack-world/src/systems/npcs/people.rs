//! NPC user operations.

use greyhack_behaviour::{
    DeviceKind, FileData, bank_config_path, hash_password, mail_config_path, npc_mail_address,
};

use crate::generation::{ComputerOptions, network};
use crate::model::{ComputerId, Gender, NetworkId, Npc, Person, PersonId};
use crate::{NpcSchedule, Result, Rng, Routine, Word, WorkPerformance, World};

impl World {
    /// Adds offline NPC `user` on `computer`, initially switching at `next_action_at`. Returns its
    /// ID, or `None` if the user is absent.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_npc(
        &mut self,
        computer: ComputerId,
        user: &str,
        name: &str,
        next_action_at: u64,
    ) -> Result<Option<PersonId>> {
        self.add_npc_with_mail_name(computer, user, user, name, next_action_at)
            .await
    }

    /// [`Self::add_npc`] whose mail address is `mail_name@domain`. The game gives it the account's own name unless a
    /// mission NPC that is already on another computer of the network takes over the account, which then has the
    /// address of the NPC it replaced (`NpcInfo.UpdateNpcInfo`'s `mailUserAlias`).
    pub(crate) async fn add_npc_with_mail_name(
        &mut self,
        computer: ComputerId,
        user: &str,
        mail_name: &str,
        name: &str,
        next_action_at: u64,
    ) -> Result<Option<PersonId>> {
        if self.db.user(computer, user).await?.is_none() {
            return Ok(None);
        }

        let mut rng = self.rng.fork("npc-accounts");
        let domain = self.mail_domain(computer).await?;
        let mut attempt = 0;
        let mut mail_address = npc_mail_address(mail_name, &domain, attempt);

        while self.mail_account(&mail_address).await?.is_some() {
            attempt += 1;
            mail_address = npc_mail_address(mail_name, &domain, attempt);
        }

        let mail_password = Word::Password.generate(&mut rng);

        self.add_mail_account(&mail_address, &mail_password).await?;

        let mut bank_number = network::bank_account(&mut rng, &domain);

        while self.bank_account(&bank_number).await?.is_some() {
            bank_number = network::bank_account(&mut rng, &domain);
        }

        let bank_password = Word::Password.generate(&mut rng);

        self.add_bank(&bank_number, &bank_password, 0.0).await?;

        // `Persona.GeneraMail`/`SetBanco` store account:number and MD5-password pairs in Config
        // (`Persona.SetBanco`, `Computer.GeneraMails`).
        self.write_file_as(
            computer,
            &mail_config_path(user),
            user,
            FileData::text(&format!("{mail_address}:{}", hash_password(&mail_password))),
        )
        .await?;
        self.write_file_as(
            computer,
            &bank_config_path(user),
            user,
            FileData::text(&format!("{bank_number}:{}", hash_password(&bank_password))),
        )
        .await?;

        let id = self
            .db
            .insert_person(&Person {
                id: PersonId(0),
                name: name.to_owned(),
                // NPC surnames match account names (`NetworkLan.AddNpcUser`).
                surname: user.to_owned(),
                gender: Gender::Male,
                mail_address: Some(mail_address),
                bank_number: Some(bank_number),
            })
            .await?;

        if let Some(mut account) = self.db.user(computer, user).await? {
            account.person = Some(id);
            self.db.save_user(computer, &account).await?;
        }

        // `NpcInfo.NpcInfo`: draws the new NPC work schedule for its assigned shift.
        let day_time = rng.range(0, 1) == 1;
        let schedule = NpcSchedule::new(&mut rng, day_time);
        // `NpcInfo.NpcInfo`: `workPerformance` is a uniform draw from `WorkStatus`, the last draw of
        // an NPC after its shift and schedule.
        let work_performance = *rng.pick(&WorkPerformance::ALL);

        self.db
            .insert_npc(id, computer, user, false, &schedule, work_performance)
            .await?;

        self.schedule(next_action_at, Routine::NpcSwitch { npc: id })
            .await?;

        Ok(Some(id))
    }

    /// Gives `person` the mail address `address` (`NpcInfo.userMail`), renaming their mail account with its password and
    /// mails. Copies a game address that is not `surname@domain`, as a mission NPC's is
    /// (`NpcInfo.UpdateNpcInfo`). Does nothing for a person without a mail account.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_person_mail_address(&mut self, person: PersonId, address: &str) -> Result<()> {
        let Some(mut row) = self.db.person(person).await? else {
            return Ok(());
        };
        let Some(old) = row.mail_address.clone() else {
            return Ok(());
        };

        self.db.rename_mail_account(&old, address).await?;
        row.mail_address = Some(address.to_owned());

        self.db.save_person(&row).await
    }

    /// Makes `npc` the network administrator with unchanged skills, matching copied Grey Hack state
    /// (`NetworkLan.adminInfo`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn appoint_network_admin(&mut self, npc: PersonId) -> Result<()> {
        let Some(row) = self.db.npc(npc).await? else {
            return Ok(());
        };
        let Some(computer) = self.db.computer(row.computer).await? else {
            return Ok(());
        };
        let Some(admin) = self.db.network_admin(computer.network).await? else {
            return Ok(());
        };

        self.db
            .set_network_admin(computer.network, Some(npc), &admin)
            .await
    }

    /// Returns whether `id` administers its network (`NetworkLan.adminInfo`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub(crate) async fn is_network_admin(&self, id: PersonId) -> Result<bool> {
        let Some(npc) = self.db.npc(id).await? else {
            return Ok(false);
        };
        let Some(computer) = self.db.computer(npc.computer).await? else {
            return Ok(false);
        };

        Ok(self.db.network_admin_person(computer.network).await? == Some(id))
    }

    /// Returns the NPC with account `user` in `network`.
    pub(crate) async fn npc_in_network(
        &self,
        network: NetworkId,
        user: &str,
    ) -> Result<Option<Npc>> {
        Ok(self
            .db
            .npcs_in(network)
            .await?
            .into_iter()
            .find(|npc| npc.user == user))
    }

    /// Returns the NPC with account `user` on `computer`.
    pub(crate) async fn npc_on_computer(
        &self,
        computer: ComputerId,
        user: &str,
    ) -> Result<Option<Npc>> {
        Ok(self
            .db
            .npcs_on(computer)
            .await?
            .into_iter()
            .find(|npc| npc.user == user))
    }

    /// Returns whether the smartphone of NPC `person` is in `network`.
    pub(super) async fn has_smartphone(
        &self,
        network: NetworkId,
        person: PersonId,
    ) -> Result<bool> {
        Ok(self
            .db
            .computers_in(network)
            .await?
            .into_iter()
            .any(|computer| {
                computer.kind == DeviceKind::Smartphone && computer.npc_owner == Some(person)
            }))
    }

    /// Returns NPC `id`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn npc(&self, id: PersonId) -> Result<Option<Npc>> {
        self.db.npc(id).await
    }

    /// Returns NPCs on `computer`, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn npcs_on(&self, computer: ComputerId) -> Result<Vec<Npc>> {
        self.db.npcs_on(computer).await
    }

    /// Adds NPC `npc`'s smartphone for account `user` on `computer`, copying their mail
    /// configuration (`NetworkLan.GenerateSmartPhone`).
    pub(in crate::systems) async fn add_npc_phone(
        &mut self,
        computer: ComputerId,
        npc: PersonId,
        user: &str,
        rng: &mut Rng,
    ) -> Result<()> {
        let phone = self
            .add_phone(
                computer,
                npc,
                &ComputerOptions {
                    users: Some(vec![(user.to_owned(), Word::Password.generate(rng))]),
                    ..ComputerOptions::default()
                },
            )
            .await?;

        if let Some(phone) = phone {
            let path = mail_config_path(user);

            if let Some(content) = self.content(computer, &path, false).await? {
                self.write_file_as(
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
}

#[cfg(test)]
#[path = "tests/people.rs"]
mod tests;
