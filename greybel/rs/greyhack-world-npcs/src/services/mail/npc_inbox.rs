//! The mail the game generates for NPC accounts.

use greyhack_world_assets::GameTextTable;
use greyhack_world_core::Result;
use mono_random::Random;

use crate::generation::npc_mail::{self, Administrator, Colleague, Owner};
use crate::{Mail, Npc};

/// Returns the mail view of `npc` (`NpcInfo`) named `surname` and `name`, with mail address `mail`.
fn colleague_of(npc: &Npc, surname: &str, name: &str, mail: String) -> Colleague {
    Colleague {
        surname: surname.to_owned(),
        first_name: name
            .strip_suffix(&format!(" {surname}"))
            .unwrap_or(name)
            .to_owned(),
        address: mail,
        job_role: npc.job_role,
        work_performance: npc.work_performance,
    }
}

/// Methods of [`World`] for npcs mail npc inbox.
impl Mail {
    pub async fn generate_npc_mail(&self, address: &str) -> Result<()> {
        if self.mail.has_npc_contents(address).await? {
            return Ok(());
        }

        let Some(person) = self.mail.mail_owner(address).await? else {
            return Ok(());
        };
        let Some(npc) = self.npcs.npc(person).await? else {
            return Ok(());
        };
        let Some(computer) = self.computers.computer(npc.computer).await? else {
            return Ok(());
        };

        // `Persona.GenerateMailsContents` ignores networks spawned by the player.
        if self
            .networks
            .topology
            .computer(self.world.player())
            .await?
            .is_some_and(|player| player.network == computer.network)
        {
            return Ok(());
        }

        let mut network = Vec::new();

        for colleague in self.npcs.npcs_in(computer.network).await? {
            if let Some(row) = self.computers.person(colleague.person).await?
                && let Some(mail) = self.mail.mail_address(colleague.person).await?
            {
                network.push(colleague_of(&colleague, &row.surname, &row.name, mail));
            }
        }

        let Some(own) = network
            .iter()
            .find(|colleague| colleague.address == address)
        else {
            return Ok(());
        };
        let own = Owner {
            colleague: own.clone(),
            break_start: npc.schedule.break_start(),
            company_align: npc.company_align,
            day_zero_contact: npc.day_zero_contact.clone(),
        };
        let administrator = match (
            self.network_admins.network_admin(computer.network).await?,
            self.network_admins
                .network_admin_person(computer.network)
                .await?,
        ) {
            (Some(stats), Some(person)) => {
                self.computers
                    .person(person)
                    .await?
                    .map(|person| Administrator {
                        surname: person.surname,
                        stats,
                    })
            }
            _ => None,
        };
        let mission_contact = self.networks.mission_contact(computer.network).await?;
        let texts = GameTextTable::english();
        let text = |key: &str| texts.get(key).unwrap_or(key).to_owned();
        // The game seeds this generator with `Guid.NewGuid().GetHashCode()`.
        let seed = self.world.world_mut().rng.fork("npc-mail").next_u64() as i32;
        let mut random = Random::new(seed);

        for delivery in npc_mail::contents(
            &mut random,
            &own,
            &network,
            administrator.as_ref(),
            mission_contact.as_deref(),
            &text,
        ) {
            if self.mail.mail_account(&delivery.to).await?.is_none() {
                continue;
            }

            let body = if delivery.sending {
                format!("> {}", delivery.text)
            } else {
                delivery.text
            };

            self.mail
                .insert_email(&delivery.to, &delivery.other, &delivery.subject, &body)
                .await?;
        }

        self.mail.set_npc_contents(address, true).await
    }
}
