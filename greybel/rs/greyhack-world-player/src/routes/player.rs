//! Global player operations (`PlayerIntrinsics`).

use greyhack_behaviour::LibraryClass;
use greyhack_world_core::routes::conversions::IntoProtocol;
use greyhack_world_core::routes::require_connection;
use greyhack_world_core::{Access, AccessHandle, AccessKind, Login, ObjectTableExt as _};
use greyhack_world_creation::ObjectTableExt as _;
use greyhack_world_economy::BlockchainHandle;
use greyhack_world_intrusion::CryptoHandle;
use greyhack_world_libraries::{MetaxploitHandle, MetaxploitRef};
use greyhack_world_npcs::{MailRef, MetaMailHandle};
use greyhack_world_programs::protocol::{
    CtfFailure, Failure, IncludedLibrary, MailFailure, ObjectOutcome, Outcome, PlayerCd,
    PlayerCurrentDate, PlayerGetCtf, PlayerGetRouter, PlayerGetShell, PlayerGetSwitch,
    PlayerIncludeLib, PlayerMailLogin, PlayerNslookup, PlayerResetCtfPassword,
    PlayerUserBankNumber, PlayerUserMailAddress, PlayerWhois, ShellLogin, Whois,
};
use greyhack_world_router::RouteTable;
use greyhack_world_services::ServiceRef;
use greyhack_world_services::objects::ObjectTableExt as _;
use greyhack_world_smart_devices::{SmartApplianceHandle, TrafficNetHandle};

use crate::{CtfPasswordFailure, CtfRef, ObjectTableExt as _, PlayerAccount};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    register_player_network::<W>(routes);
    register_player_files::<W>(routes);
    register_player_accounts::<W>(routes);
    register_player_mail::<W>(routes);
    register_player_ctf::<W>(routes);
}

fn register_player_network<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerGetRouter<'static>| {
            let world = &*ctx.modules;
            let computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .computer;

            require_connection(world, &ctx.terminal_computers(call)).await?;

            let router = world
                .module::<crate::PlayerModule>()
                .player
                .network_router(computer, &ctx.terminal_computers(call), &message.address)
                .await?;

            Ok(router.map(|router| ctx.trees.objects(call.tree).add_router(router)))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerGetSwitch<'static>| {
            let world = &*ctx.modules;
            let computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .computer;
            let router = world
                .module::<crate::PlayerModule>()
                .player
                .lan_switch(computer, &message.address)
                .await?;

            Ok(router.map(|router| ctx.trees.objects(call.tree).add_router(router)))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: PlayerNslookup<'static>| {
            let world = &*ctx.modules;

            Ok(world
                .module::<greyhack_world_core::CoreModule>()
                .networks
                .dns
                .resolve_domain(&message.address)
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: PlayerWhois<'static>| {
            let world = &*ctx.modules;

            Ok(
                match world
                    .module::<greyhack_world_core::CoreModule>()
                    .networks
                    .dns
                    .whois(&message.address)
                    .await?
                {
                    None => Whois::UnknownAddress,
                    Some((_, None)) => Whois::NoInfo,
                    Some((domain, Some(contact))) => Whois::Contact {
                        domain,
                        name: contact.name,
                        mail: contact.mail,
                        phone: contact.phone,
                    },
                },
            )
        },
    );
}

fn register_player_files<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerCd<'static>| {
            let world = &*ctx.modules;
            let session = &ctx.trees.objects(call.tree).session(
                ctx.session(call)
                    .expect("dispatch checked the active program"),
            )?;

            Ok(world
                .module::<crate::PlayerModule>()
                .player
                .change_folder(
                    session.computer,
                    &session.user,
                    &message.current,
                    &message.path,
                )
                .await?
                .map(IntoProtocol::into_protocol))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    _message: PlayerCurrentDate| {
            let world = &*ctx.modules;

            Ok(world
                .module::<greyhack_world_core::CoreModule>()
                .world
                .clone()
                .date())
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerIncludeLib<'static>| {
            let world = &*ctx.modules;
            let access = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .clone();
            let Some(library) = world
                .module::<crate::PlayerModule>()
                .player
                .included_library(access.computer, &message.path)
                .await?
            else {
                return Ok(IncludedLibrary::Null);
            };

            Ok(match library.class {
                LibraryClass::Metaxploit => {
                    IncludedLibrary::Metaxploit(ctx.trees.objects(call.tree).insert(
                        MetaxploitHandle(MetaxploitRef {
                            access,
                            version: library.version,
                        }),
                    ))
                }
                LibraryClass::Crypto => IncludedLibrary::Crypto(
                    ctx.trees.objects(call.tree).insert(CryptoHandle(access)),
                ),
                LibraryClass::AptClient => {
                    IncludedLibrary::AptClient(ctx.trees.objects(call.tree).add_apt_client(access))
                }
                LibraryClass::Blockchain => IncludedLibrary::Blockchain(
                    ctx.trees
                        .objects(call.tree)
                        .insert(BlockchainHandle(access)),
                ),
                LibraryClass::SmartAppliance => IncludedLibrary::SmartAppliance(
                    ctx.trees
                        .objects(call.tree)
                        .insert(SmartApplianceHandle(access)),
                ),
                LibraryClass::TrafficNet => IncludedLibrary::TrafficNet(
                    ctx.trees
                        .objects(call.tree)
                        .insert(TrafficNetHandle(access)),
                ),
                LibraryClass::Service(service) => IncludedLibrary::Service(
                    ctx.trees
                        .objects(call.tree)
                        .add_service(ServiceRef { access, service }),
                ),
            })
        },
    );
}

fn register_player_accounts<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerGetShell<'static>| {
            let world = &*ctx.modules;
            let session = &ctx.trees.objects(call.tree).session(
                ctx.session(call)
                    .expect("dispatch checked the active program"),
            )?;
            let access = Access {
                computer: session.computer,
                user: message.user.to_string(),
                ftp: false,
                // `PlayerIntrinsics.get_shell` uses `GreyInterpreter.GetLastPrevBounceIP`, the previous
                // address of the program's current shell.
                prev_ip: session.prev_ip.clone(),
                bounces: session.bounces,
            };

            Ok(
                match world
                    .module::<greyhack_world_core::CoreModule>()
                    .computers
                    .users
                    .check_login(access.computer, &message.user, &message.password)
                    .await?
                {
                    Login::Valid => {
                        ShellLogin::Shell(ctx.trees.objects(call.tree).insert(AccessHandle {
                            access,
                            kind: AccessKind::Shell,
                        }))
                    }
                    Login::InvalidCredentials => ShellLogin::Null,
                    Login::AccountInactive => ShellLogin::AccountInactive,
                },
            )
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    _message: PlayerUserMailAddress| {
            let world = &*ctx.modules;
            let computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .computer;

            Ok(world
                .module::<crate::PlayerModule>()
                .player
                .player_account(computer, PlayerAccount::MailAddress)
                .await?)
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    _message: PlayerUserBankNumber| {
            let world = &*ctx.modules;
            let computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .computer;

            Ok(world
                .module::<crate::PlayerModule>()
                .player
                .player_account(computer, PlayerAccount::BankNumber)
                .await?)
        },
    );
}

fn register_player_mail<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerMailLogin<'static>| {
            let world = &*ctx.modules;
            let access = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .clone();

            Ok(
                match world
                    .module::<crate::PlayerModule>()
                    .player
                    .mail_login_from(
                        access.computer,
                        &ctx.terminal_computers(call),
                        &message.address,
                        &message.password,
                    )
                    .await?
                {
                    Some(Login::Valid) => {
                        let emails = world
                            .module::<greyhack_world_npcs::NpcModule>()
                            .mail
                            .mail_account(&message.address)
                            .await?
                            .map_or_else(Vec::new, |account| account.inbox);

                        ObjectOutcome::Object(ctx.trees.objects(call.tree).insert(MetaMailHandle(
                            MailRef {
                                access,
                                address: message.address.to_string(),
                                emails,
                            },
                        )))
                    }
                    None => ObjectOutcome::Failure(Failure::Mail(MailFailure::NoInternet)),
                    Some(Login::InvalidCredentials) => {
                        ObjectOutcome::Failure(Failure::Mail(MailFailure::WrongCredentials))
                    }
                    Some(Login::AccountInactive) => {
                        ObjectOutcome::Failure(Failure::Mail(MailFailure::AccountInactive))
                    }
                },
            )
        },
    );
}

fn register_player_ctf<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerGetCtf<'static>| {
            let world = &*ctx.modules;
            let computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .computer;

            Ok(world
                .module::<crate::PlayerModule>()
                .player
                .ctf_login(&message.event, &message.user, &message.password)
                .await?
                .then(|| {
                    ctx.trees.objects(call.tree).add_ctf(CtfRef {
                        computer,
                        event: message.event.to_string(),
                    })
                }))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: PlayerResetCtfPassword<'static>| {
            let world = &*ctx.modules;
            let computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .computer;

            Ok(
                match world
                    .module::<crate::PlayerModule>()
                    .player
                    .change_ctf_password(computer, &message.password)
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(CtfPasswordFailure::NoAccount) => {
                        Outcome::Failure(Failure::Ctf(CtfFailure::NoAccount))
                    }
                    Err(CtfPasswordFailure::Refusal(refusal)) => {
                        Outcome::Failure(Failure::Refusal(refusal.into_protocol()))
                    }
                },
            )
        },
    );
}
