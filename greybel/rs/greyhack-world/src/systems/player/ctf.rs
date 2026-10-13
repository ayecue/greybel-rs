//! CTF account, event, participation, and rating operations.

use greyhack_behaviour::{MARK_RANGE, ROOT_USER, creator_reward, creator_score, player_reward};

use crate::model::{
    ComputerId, CtfAccount, CtfEvent, CtfFailure, CtfPlayer, Gender, Person, PersonId,
};
use crate::{Result, World};

impl World {
    /// Returns the person owning CTF data for `computer`: an existing account owner or a new person
    /// owning the first non-root account.
    pub(crate) async fn person_for(&mut self, computer: ComputerId) -> Result<Option<PersonId>> {
        if let Some(person) = self.db.person_on(computer).await? {
            return Ok(Some(person));
        }

        let users = self.db.users(computer).await?;
        let Some(mut user) = users
            .iter()
            .find(|user| user.name != ROOT_USER)
            .or_else(|| users.first())
            .cloned()
        else {
            return Ok(None);
        };
        let person = self
            .db
            .insert_person(&Person {
                id: PersonId(0),
                name: user.name.clone(),
                surname: String::new(),
                gender: Gender::Male,
                mail_address: None,
                bank_number: None,
            })
            .await?;

        user.person = Some(person);
        self.db.save_user(computer, &user).await?;

        Ok(Some(person))
    }

    /// Creates or replaces the CTF account for `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn create_ctf_account(
        &mut self,
        computer: ComputerId,
        user: &str,
        password: &str,
    ) -> Result<()> {
        let Some(person) = self.person_for(computer).await? else {
            return Ok(());
        };
        let password_hash = self.db.register_password(password).await?;

        self.db
            .set_ctf_account(&CtfAccount {
                person,
                user: user.to_owned(),
                password_hash,
            })
            .await
    }

    /// Returns the CTF account for `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn ctf_account(&self, computer: ComputerId) -> Result<Option<CtfAccount>> {
        match self.db.person_on(computer).await? {
            Some(person) => self.db.ctf_account(person).await,
            None => Ok(None),
        }
    }

    /// Creates or replaces CTF `event`; rating totals derive from players.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn create_ctf_event(&mut self, event: &CtfEvent) -> Result<()> {
        self.db.set_ctf_event(event).await
    }

    /// Returns CTF event `name`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn ctf_event(&self, name: &str) -> Result<Option<CtfEvent>> {
        self.db.ctf_event(name).await
    }

    /// Returns players of CTF event `name`, ordered by computer.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn ctf_players(&self, name: &str) -> Result<Vec<CtfPlayer>> {
        self.db.ctf_players(name).await
    }

    /// Adds `computer` to CTF event `name`. Returns false if the event is absent or the computer
    /// already plays.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_ctf_player(&mut self, name: &str, computer: ComputerId) -> Result<bool> {
        let Some(person) = self.person_for(computer).await? else {
            return Ok(false);
        };

        if self.db.ctf_event(name).await?.is_none()
            || self.db.ctf_player(name, person).await?.is_some()
        {
            return Ok(false);
        }

        self.db
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

    /// Marks `computer`'s participation in CTF event `name` complete and sends mail. Returns false
    /// if not playing or already complete.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn player_success(&mut self, name: &str, computer: ComputerId) -> Result<bool> {
        let Some(person) = self.db.person_on(computer).await? else {
            return Ok(false);
        };
        let Some(mut player) = self.db.ctf_player(name, person).await? else {
            return Ok(false);
        };

        if player.completed {
            return Ok(false);
        }

        player.completed = true;
        self.db.set_ctf_player(&player).await?;
        self.deliver_mail(
            &player.mail_user,
            "ctfevent@noreply.com",
            "CTF completed!",
            "You have successfully completed the mission!\nEvaluate the mission at the hackshop to receive the reward.",
        )
        .await?;

        Ok(true)
    }

    /// Rates CTF event `name` from `computer`, pays player and creator rewards, and returns the
    /// rewards or rating failure.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_ctf_score(
        &mut self,
        name: &str,
        computer: ComputerId,
        quality: i64,
        difficulty: i64,
    ) -> Result<Result<(i64, i64), CtfFailure>> {
        if !MARK_RANGE.contains(&quality) || !MARK_RANGE.contains(&difficulty) {
            return Ok(Err(CtfFailure::InvalidScore));
        }

        let Some(mut event) = self.db.ctf_event(name).await? else {
            return Ok(Err(CtfFailure::UnknownEvent));
        };
        let Some(person) = self.db.person_on(computer).await? else {
            return Ok(Err(CtfFailure::Incomplete));
        };
        let Some(mut player) = self.db.ctf_player(name, person).await? else {
            return Ok(Err(CtfFailure::Incomplete));
        };

        if !player.completed {
            return Ok(Err(CtfFailure::Incomplete));
        }

        if player.difficulty_rate > 0 {
            return Ok(Err(CtfFailure::RepeatScore));
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
        self.db.set_ctf_event(&event).await?;

        // A creator may rate their own event; it counts, and they are removed from its players.
        if event.creator == person {
            self.db.delete_ctf_player(name, person).await?;
        } else {
            player.quality_rate = quality;
            player.difficulty_rate = difficulty;
            self.db.set_ctf_player(&player).await?;
        }

        let creator_score = self.creator_score(event.creator).await?;
        let creator_reward = creator_reward(creator_score, event.quality_rank, event.num_votes);

        self.deposit(&player.bank_user, player_reward as f64)
            .await?;
        self.deposit(&event.creator_bank_user, creator_reward as f64)
            .await?;

        Ok(Ok((player_reward, creator_reward)))
    }

    /// Returns the creator's mean quality score across events.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn ctf_creator_score(&self, owner: ComputerId) -> Result<f32> {
        match self.db.person_on(owner).await? {
            Some(person) => self.creator_score(person).await,
            None => Ok(0.0),
        }
    }

    async fn creator_score(&self, creator: PersonId) -> Result<f32> {
        let events = self.db.ctf_events_of(creator).await?;
        let ratings: Vec<_> = events
            .iter()
            .map(|event| (event.quality_rank, event.num_votes))
            .collect();

        Ok(creator_score(&ratings))
    }

    /// Changes the CTF password for `computer` and its created events. Returns false if no CTF
    /// account exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn reset_ctf_password(
        &mut self,
        computer: ComputerId,
        password: &str,
    ) -> Result<bool> {
        let Some(person) = self.db.person_on(computer).await? else {
            return Ok(false);
        };
        let Some(mut account) = self.db.ctf_account(person).await? else {
            return Ok(false);
        };

        account.password_hash = self.db.register_password(password).await?;
        self.db.set_ctf_account(&account).await?;

        for mut event in self.db.ctf_events_of(person).await? {
            event.password_hash.clone_from(&account.password_hash);
            self.db.set_ctf_event(&event).await?;
        }

        Ok(true)
    }
}
