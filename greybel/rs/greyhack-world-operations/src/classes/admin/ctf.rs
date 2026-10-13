//! CTF events and debug accounts (`admin.so`).

use greyhack_behaviour::hash_password;
use greyhack_world::{ComputerId, CtfEvent, DebugAccount, World};

use crate::Result;
use crate::protocol::{CtfEventOptions, CtfRating, Failure};

/// Creates a CTF event whose creator and initial player belong to `computer`.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn create_ctf_event(
    world: &mut World,
    computer: ComputerId,
    event: CtfEventOptions,
) -> Result<()> {
    let name = event.name;

    // Resolve or create the computer's person with `add_ctf_player` before validating the event.
    world.add_ctf_player(&name, computer).await?;

    let Some(creator) = world.person_on(computer).await? else {
        return Ok(());
    };
    let creator_bank_user = world.bank_number_of(creator).await?.unwrap_or_default();

    world
        .create_ctf_event(&CtfEvent {
            name: name.clone(),
            user: event.user,
            password_hash: hash_password(&event.password),
            description: event.description,
            public_address: event.public_address,
            creator_name: event.creator_name,
            mail_content: event.mail_content,
            creator,
            creator_bank_user,
            quality_rank: 3,
            difficulty_rank: 3,
            num_votes: 1,
            is_rated: false,
        })
        .await?;
    world.add_ctf_player(&name, computer).await?;

    Ok(())
}

/// Submits quality and difficulty ratings for `name` from `computer`'s player.
pub(super) async fn rate_ctf_event(
    world: &mut World,
    computer: ComputerId,
    name: &str,
    quality: i64,
    difficulty: i64,
) -> Result<CtfRating> {
    Ok(
        match world
            .add_ctf_score(name, computer, quality, difficulty)
            .await?
        {
            Ok((participant, creator)) => CtfRating::Rewards {
                participant,
                creator,
            },
            Err(failure) => CtfRating::Failure(Failure::Ctf(failure.into())),
        },
    )
}

/// Creates or updates the debug account for `user`.
pub(super) async fn create_debug_account(
    world: &mut World,
    user: &str,
    password: &str,
) -> Result<()> {
    world
        .save_debug_account(&DebugAccount::new(user, password))
        .await?;

    Ok(())
}
