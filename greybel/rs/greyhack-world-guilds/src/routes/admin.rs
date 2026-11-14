//! Guild operations of the global `admin.so` program.

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{
    AdminCreateGuild, AdminHireCrownShield, AdminSetCompanyShares, Failure, Outcome,
};

use crate::routes::protocol::IntoProtocol as _;
use crate::{GuildContext, GuildDraft};

pub(crate) fn register<W: GuildContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminCreateGuild<'static>| {
            Ok(
                match ctx
                    .modules
                    .guilds()
                    .guilds
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
                    Err(failure) => Outcome::Failure(Failure::Admin(failure.into_protocol())),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminSetCompanyShares<'static>| {
            Ok(ctx
                .modules
                .guilds()
                .guilds
                .set_company_shares(&message.public_ip, &message.bank_account, message.shares)
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminHireCrownShield<'static>| {
            Ok(
                match ctx
                    .modules
                    .guilds()
                    .guilds
                    .hire_crown_shield(&message.guild, &message.public_ip)
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(failure) => Outcome::Failure(Failure::Admin(failure.into_protocol())),
                },
            )
        },
    );
}
