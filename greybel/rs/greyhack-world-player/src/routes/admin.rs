//! Player operations of the global `admin.so` program: missions and CTF events.

use greyhack_behaviour::hash_password;
use greyhack_world_core::routes::conversions::IntoProtocol as _;
use greyhack_world_core::{ComputerId, ObjectTableExt as _};
use greyhack_world_programs::protocol::{
    AdminAcceptMission, AdminCreateCtfAccount, AdminCreateCtfEvent, AdminRateCtfEvent,
    CtfEventOptions, CtfRating, Failure, MissionAcceptance,
};
use greyhack_world_router::RouteTable;

use crate::CtfEvent;
use crate::routes::Result;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminAcceptMission<'static>| {
            let world = &*ctx.modules;

            Ok(
                match world
                    .module::<crate::PlayerModule>()
                    .mission_offers
                    .accept_procedural_mission(&message.shop_ip)
                    .await?
                {
                    None => MissionAcceptance::Null,
                    Some(Ok(mission)) => MissionAcceptance::Accepted(mission.group[0].clone()),
                    Some(Err(failure)) => {
                        MissionAcceptance::Failure(Failure::Mission(failure.into_protocol()))
                    }
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AdminCreateCtfAccount<'static>| {
            let world = &*ctx.modules;

            world
                .module::<crate::PlayerModule>()
                .ctf
                .create_ctf_account(
                    ctx.trees
                        .objects(call.tree)
                        .session(
                            ctx.session(call)
                                .expect("dispatch checked the active program"),
                        )?
                        .computer,
                    &message.user,
                    &message.password,
                )
                .await?;

            Ok(true)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AdminCreateCtfEvent| {
            let world = &*ctx.modules;

            create_ctf_event(
                world,
                ctx.trees
                    .objects(call.tree)
                    .session(
                        ctx.session(call)
                            .expect("dispatch checked the active program"),
                    )?
                    .computer,
                message.event,
            )
            .await?;

            Ok(true)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AdminRateCtfEvent<'static>| {
            let world = &*ctx.modules;

            rate_ctf_event(
                world,
                ctx.trees
                    .objects(call.tree)
                    .session(
                        ctx.session(call)
                            .expect("dispatch checked the active program"),
                    )?
                    .computer,
                &message.name,
                message.quality,
                message.difficulty,
            )
            .await
        },
    );
}

/// Creates a CTF event whose creator and initial player belong to `computer`.
///
/// # Errors
///
/// Returns errors from database reads.
async fn create_ctf_event(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    computer: ComputerId,
    event: CtfEventOptions,
) -> Result<()> {
    let name = event.name;

    // Resolve or create the computer's person with `add_ctf_player` before validating the event.
    world
        .module::<crate::PlayerModule>()
        .ctf
        .add_ctf_player(&name, computer)
        .await?;

    let Some(creator) = world
        .module::<greyhack_world_npcs::NpcModule>()
        .npcs
        .person_on(computer)
        .await?
    else {
        return Ok(());
    };
    let creator_bank_user = world
        .module::<greyhack_world_economy::EconomyModule>()
        .bank
        .bank_number_of(creator)
        .await?
        .unwrap_or_default();

    world
        .module::<crate::PlayerModule>()
        .ctf
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
    world
        .module::<crate::PlayerModule>()
        .ctf
        .add_ctf_player(&name, computer)
        .await?;

    Ok(())
}

/// Submits quality and difficulty ratings for `name` from `computer`'s player.
async fn rate_ctf_event(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    computer: ComputerId,
    name: &str,
    quality: i64,
    difficulty: i64,
) -> Result<CtfRating> {
    Ok(
        match world
            .module::<crate::PlayerModule>()
            .ctf
            .add_ctf_score(name, computer, quality, difficulty)
            .await?
        {
            Ok((participant, creator)) => CtfRating::Rewards {
                participant,
                creator,
            },
            Err(failure) => CtfRating::Failure(Failure::Ctf(failure.into_protocol())),
        },
    )
}
