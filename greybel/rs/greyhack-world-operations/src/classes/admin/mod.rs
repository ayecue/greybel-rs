//! Global `admin.so` operations (`admin`) for clocks, networks, accounts, money, CTF events, and snapshots.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::ROOT_USER;
use greyhack_world::GuildDraft;

use crate::OperationError;
use crate::connection::Connection;
use crate::connection::objects::{Access, Object};
use crate::protocol::{
    AdminAcceptMission, AdminBuyExploit, AdminCreateCtfAccount, AdminCreateCtfEvent,
    AdminCreateDebugAccount, AdminCreateGuild, AdminCreateMailAccount, AdminCreateNetwork,
    AdminCreateProgram, AdminDeposit, AdminExploitShop, AdminGrantCoins, AdminHireCrownShield,
    AdminNetworks, AdminRateCtfEvent, AdminRegisterPassword, AdminSeed, AdminSetCompanyShares,
    AdminSetDate, AdminSnapshot, AdminSudo, AdminTick, AdminTrafficAccident, Failure, GameServer,
    MissionAcceptance, NetworkCreation, Outcome,
};

mod ctf;
mod networks;
mod shop;
use self::ctf::{create_ctf_event, create_debug_account, rate_ctf_event};
pub(crate) use self::networks::Creation;
use self::networks::{create_network, create_program};
use self::shop::{buy_exploit, exploit_shop, grant_coins};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(channel, async |world, _, _: AdminSeed, _| Ok(world.seed));
    connection.change(channel, async |world, _, _: AdminTrafficAccident, _| {
        Ok(world.traffic_accident().await?)
    });
    connection.read(channel, async |world, _, _: AdminNetworks, _| {
        Ok(world.public_addresses().await?)
    });
    connection.change(
        channel,
        async |world, objects, message: AdminCreateNetwork, _| {
            Ok(match create_network(world, message.network).await? {
                Creation::Object(router) => {
                    NetworkCreation::Router(objects.add(Object::Router(router)))
                }
                Creation::Failure(failure) => NetworkCreation::Failure(Failure::Admin(failure)),
            })
        },
    );
    connection.operate(channel, async |state, connection, message: AdminTick| {
        let target = state
            .cycle_for(connection.id, message.program, async |world_state| {
                state.session(connection.id, message.program)?;

                Ok::<_, OperationError>(
                    world_state
                        .world
                        .time_seconds()
                        .saturating_add(message.seconds),
                )
            })
            .await?;
        let budget = state.routine_budget();

        // Advance each cycle to the next due routine or the target, then run due routines.
        while !state
            .cycle(async |world_state| world_state.world.advance_step(target, budget).await)
            .await?
        {}

        Ok(state
            .cycle(async |world_state| world_state.world.time_seconds())
            .await)
    });
    connection.change(
        channel,
        async |world, _, message: AdminSetDate<'static>, _| Ok(world.set_date(&message.date)),
    );
    connection.operate(channel, async |state, connection, message: AdminSudo| {
        state
            .cycle(async |world_state| {
                let session = state.session(connection.id, message.program)?;
                let (_, objects) = world_state.of(connection.id);
                let session = objects.session(session)?;
                let root = Access {
                    computer: session.computer,
                    user: ROOT_USER.to_owned(),
                    ftp: false,
                    prev_ip: session.prev_ip.clone(),
                    bounces: session.bounces,
                };
                let shell = objects.add(Object::Shell(root));

                state.set_session(connection.id, message.program, shell);

                Ok(shell)
            })
            .await
    });
    connection.change(
        channel,
        async |world, objects, message: AdminCreateProgram<'static>, caller| {
            let session = objects.session(caller.session)?;

            create_program(
                world,
                session.computer,
                &session.user,
                &message.path,
                &message.source,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminExploitShop<'static>, _| {
            exploit_shop(world, &message.library, &message.version).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: AdminBuyExploit<'static>, caller| {
            let session = objects.session(caller.session)?;

            buy_exploit(
                world,
                session.computer,
                &session.user,
                &caller.terminal_computers,
                &message,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminGrantCoins<'static>, _| {
            grant_coins(world, &message.wallet_user, &message.coin, message.amount).await
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminDeposit<'static>, _| {
            Ok(world.deposit(&message.number, message.amount).await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminAcceptMission<'static>, _| {
            Ok(
                match world.accept_procedural_mission(&message.shop_ip).await? {
                    None => MissionAcceptance::Null,
                    Some(Ok(mission)) => MissionAcceptance::Accepted(mission.group[0].clone()),
                    Some(Err(failure)) => {
                        MissionAcceptance::Failure(Failure::Mission(failure.into()))
                    }
                },
            )
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminCreateMailAccount<'static>, _| {
            world
                .add_mail_account(&message.address, &message.password)
                .await?;

            Ok(true)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: AdminCreateCtfAccount<'static>, caller| {
            world
                .create_ctf_account(
                    objects.session(caller.session)?.computer,
                    &message.user,
                    &message.password,
                )
                .await?;

            Ok(true)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: AdminCreateCtfEvent, caller| {
            create_ctf_event(
                world,
                objects.session(caller.session)?.computer,
                message.event,
            )
            .await?;

            Ok(true)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: AdminRateCtfEvent<'static>, caller| {
            rate_ctf_event(
                world,
                objects.session(caller.session)?.computer,
                &message.name,
                message.quality,
                message.difficulty,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminCreateDebugAccount<'static>, _| {
            create_debug_account(world, &message.user, &message.password).await?;

            Ok(true)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminCreateGuild<'static>, _| {
            Ok(
                match world
                    .create_guild(&GuildDraft {
                        name: message.name.to_string(),
                        bank_account: message.bank_account.to_string(),
                        total_exp: message.total_exp,
                        investment_perks: message.investment_perks,
                        admin_restore_perks: message.admin_restore_perks,
                        player_member: message.join,
                    })
                    .await?
                {
                    Ok(_) => Outcome::True,
                    Err(failure) => Outcome::Failure(Failure::Admin(failure.into())),
                },
            )
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminSetCompanyShares<'static>, _| {
            Ok(world
                .set_company_shares(&message.public_ip, &message.bank_account, message.shares)
                .await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminHireCrownShield<'static>, _| {
            Ok(
                match world
                    .hire_crown_shield(&message.guild, &message.public_ip)
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(failure) => Outcome::Failure(Failure::Admin(failure.into())),
                },
            )
        },
    );
    connection.operate(channel, async |state, _, _: AdminSnapshot| {
        Ok(state.save().await)
    });
    connection.change(
        channel,
        async |world, _, message: AdminRegisterPassword<'static>, _| {
            Ok(world.register_password(&message.password).await?)
        },
    );
}
