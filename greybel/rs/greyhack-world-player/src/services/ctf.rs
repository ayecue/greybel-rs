//! CTF account, event, participation, and rating operations.

use greyhack_behaviour::{MARK_RANGE, ROOT_USER, creator_reward, creator_score, player_reward};
use greyhack_world_core::{ComputerId, Gender, Person, PersonId, Result};

use crate::model::{CtfAccount, CtfEvent, CtfFailure, CtfPlayer};

/// Methods of [`World`] for player ctf.
impl Ctf {
    pub async fn person_for(&self, computer: ComputerId) -> Result<Option<PersonId>> {
        if let Some(person) = self.computers.person_on(computer).await? {
            return Ok(Some(person));
        }

        let users = self.computers.users(computer).await?;
        let Some(mut user) = users
            .iter()
            .find(|user| user.name != ROOT_USER)
            .or_else(|| users.first())
            .cloned()
        else {
            return Ok(None);
        };
        let person = self
            .computers
            .insert_person(&Person {
                id: PersonId(0),
                name: user.name.clone(),
                surname: String::new(),
                gender: Gender::Male,
            })
            .await?;

        user.person = Some(person);
        self.computers.save_user(computer, &user).await?;

        Ok(Some(person))
    }

    pub async fn create_ctf_account(
        &self,
        computer: ComputerId,
        user: &str,
        password: &str,
    ) -> Result<()> {
        let Some(person) = self.person_for(computer).await? else {
            return Ok(());
        };
        let password_hash = self.computers.register_password(password).await?;

        self.ctf
            .set_ctf_account(&CtfAccount {
                person,
                user: user.to_owned(),
                password_hash,
            })
            .await
    }

    pub async fn ctf_account(&self, computer: ComputerId) -> Result<Option<CtfAccount>> {
        match self.computers.person_on(computer).await? {
            Some(person) => self.ctf.ctf_account(person).await,
            None => Ok(None),
        }
    }

    pub async fn create_ctf_event(&self, event: &CtfEvent) -> Result<()> {
        self.ctf.set_ctf_event(event).await
    }

    pub async fn ctf_event(&self, name: &str) -> Result<Option<CtfEvent>> {
        self.ctf.ctf_event(name).await
    }

    pub async fn ctf_players(&self, name: &str) -> Result<Vec<CtfPlayer>> {
        self.ctf.ctf_players(name).await
    }

    pub async fn add_ctf_player(&self, name: &str, computer: ComputerId) -> Result<bool> {
        let Some(person) = self.person_for(computer).await? else {
            return Ok(false);
        };

        if self.ctf.ctf_event(name).await?.is_none()
            || self.ctf.ctf_player(name, person).await?.is_some()
        {
            return Ok(false);
        }

        self.ctf
            .set_ctf_player(&CtfPlayer {
                event: name.to_owned(),
                person,
                mail_user: String::new(),
                bank_user: String::new(),
                completed: false,
                quality_rate: 0,
                difficulty_rate: 0,
            })
            .await?;

        Ok(true)
    }

    pub async fn player_success(&self, name: &str, computer: ComputerId) -> Result<bool> {
        let Some(person) = self.computers.person_on(computer).await? else {
            return Ok(false);
        };
        let Some(mut player) = self.ctf.ctf_player(name, person).await? else {
            return Ok(false);
        };

        if player.completed {
            return Ok(false);
        }

        player.completed = true;
        self.ctf.set_ctf_player(&player).await?;
        self.mail.deliver_mail(
            &player.mail_user,
            "ctfevent@noreply.com",
            "CTF completed!",
            "You have successfully completed the mission!\nEvaluate the mission at the hackshop to receive the reward.",
        )
        .await?;

        Ok(true)
    }

    pub async fn add_ctf_score(
        &self,
        name: &str,
        computer: ComputerId,
        quality: i64,
        difficulty: i64,
    ) -> Result<Result<(i64, i64), CtfFailure>> {
        if !MARK_RANGE.contains(&quality) || !MARK_RANGE.contains(&difficulty) {
            return Ok(Err(CtfFailure::InvalidScore));
        }

        let Some(mut event) = self.ctf.ctf_event(name).await? else {
            return Ok(Err(CtfFailure::UnknownEvent));
        };
        let Some(person) = self.computers.person_on(computer).await? else {
            return Ok(Err(CtfFailure::Incomplete));
        };
        let Some(mut player) = self.ctf.ctf_player(name, person).await? else {
            return Ok(Err(CtfFailure::Incomplete));
        };

        if !player.completed {
            return Ok(Err(CtfFailure::Incomplete));
        }

        if player.difficulty_rate > 0 {
            return Ok(Err(CtfFailure::RepeatScore));
        }

        if event.creator == person {
            return Ok(Err(CtfFailure::OwnEvent));
        }

        let player_reward =
            player_reward(event.quality_rank, event.difficulty_rank, event.num_votes);

        // The first rating starts the defaults of 3, 3 and 1 over.
        if !event.is_rated {
            event.quality_rank = 0;
            event.difficulty_rank = 0;
            event.num_votes = 0;
            event.is_rated = true;
        }

        event.quality_rank += quality;
        event.difficulty_rank += difficulty;
        event.num_votes += 1;
        self.ctf.set_ctf_event(&event).await?;

        player.quality_rate = quality;
        player.difficulty_rate = difficulty;
        self.ctf.set_ctf_player(&player).await?;

        let creator_score = self.creator_score(event.creator).await?;
        let creator_reward = creator_reward(creator_score, event.quality_rank, event.num_votes);

        self.bank
            .deposit(&player.bank_user, player_reward as f64)
            .await?;
        self.bank
            .deposit(&event.creator_bank_user, creator_reward as f64)
            .await?;

        Ok(Ok((player_reward, creator_reward)))
    }

    pub async fn ctf_creator_score(&self, owner: ComputerId) -> Result<f32> {
        match self.computers.person_on(owner).await? {
            Some(person) => self.creator_score(person).await,
            None => Ok(0.0),
        }
    }

    pub async fn reset_ctf_password(&self, computer: ComputerId, password: &str) -> Result<bool> {
        let Some(person) = self.computers.person_on(computer).await? else {
            return Ok(false);
        };
        let Some(mut account) = self.ctf.ctf_account(person).await? else {
            return Ok(false);
        };

        account.password_hash = self.computers.register_password(password).await?;
        self.ctf.set_ctf_account(&account).await?;

        for mut event in self.ctf.ctf_events_of(person).await? {
            event.password_hash.clone_from(&account.password_hash);
            self.ctf.set_ctf_event(&event).await?;
        }

        Ok(true)
    }
}

impl Ctf {
    pub async fn creator_score(&self, creator: PersonId) -> Result<f32> {
        let events = self.ctf.ctf_events_of(creator).await?;
        let ratings: Vec<_> = events
            .iter()
            .map(|event| (event.quality_rank, event.num_votes))
            .collect();

        Ok(creator_score(&ratings))
    }
}

/// A world service with only its declared dependencies.
pub struct Ctf {
    pub(crate) ctf: std::rc::Rc<crate::storage::CtfRepository>,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) bank: greyhack_world_core::Dep<greyhack_world_economy::Bank>,
    pub(crate) mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
}

impl Ctf {
    pub(crate) fn new(
        ctf: std::rc::Rc<crate::storage::CtfRepository>,
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
        bank: greyhack_world_core::Dep<greyhack_world_economy::Bank>,
        mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
    ) -> Self {
        Self {
            ctf,
            computers,
            bank,
            mail,
        }
    }
}
