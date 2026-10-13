//! Global player operations (`PlayerIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::LibraryClass;
use greyhack_world::{CtfPasswordFailure, Login, PlayerAccount};

use super::require_connection;
use crate::connection::Connection;
use crate::connection::objects::{Access, CtfRef, MailRef, MetaxploitRef, Object, ServiceRef};
use crate::protocol::{
    CtfFailure, Failure, GameServer, IncludedLibrary, MailFailure, ObjectOutcome, Outcome,
    PlayerCd, PlayerCurrentDate, PlayerGetCtf, PlayerGetRouter, PlayerGetShell, PlayerGetSwitch,
    PlayerIncludeLib, PlayerMailLogin, PlayerNslookup, PlayerResetCtfPassword,
    PlayerUserBankNumber, PlayerUserMailAddress, PlayerWhois, ShellLogin, Whois,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.change(
        channel,
        async |world, objects, message: PlayerGetRouter<'static>, caller| {
            let computer = objects.session(caller.session)?.computer;

            require_connection(world, &caller.terminal_computers).await?;

            let router = world
                .network_router(computer, &caller.terminal_computers, &message.address)
                .await?;

            Ok(router.map(|router| objects.add(Object::Router(router))))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: PlayerGetSwitch<'static>, caller| {
            let computer = objects.session(caller.session)?.computer;
            let router = world.lan_switch(computer, &message.address).await?;

            Ok(router.map(|router| objects.add(Object::Router(router))))
        },
    );
    connection.read(
        channel,
        async |world, _, message: PlayerNslookup<'static>, _| {
            Ok(world.resolve_domain(&message.address).await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: PlayerWhois<'static>, _| {
            Ok(match world.whois(&message.address).await? {
                None => Whois::UnknownAddress,
                Some((_, None)) => Whois::NoInfo,
                Some((domain, Some(contact))) => Whois::Contact {
                    domain,
                    name: contact.name,
                    mail: contact.mail,
                    phone: contact.phone,
                },
            })
        },
    );
    connection.read(
        channel,
        async |world, objects, message: PlayerCd<'static>, caller| {
            let session = objects.session(caller.session)?;

            Ok(world
                .change_folder(
                    session.computer,
                    &session.user,
                    &message.current,
                    &message.path,
                )
                .await?
                .map(Into::into))
        },
    );
    connection.read(channel, async |world, _, _: PlayerCurrentDate, _| {
        Ok(world.date())
    });
    connection.read(
        channel,
        async |world, objects, message: PlayerIncludeLib<'static>, caller| {
            let access = objects.session(caller.session)?.clone();
            let Some(library) = world
                .included_library(access.computer, &message.path)
                .await?
            else {
                return Ok(IncludedLibrary::Null);
            };

            Ok(match library.class {
                LibraryClass::Metaxploit => {
                    IncludedLibrary::Metaxploit(objects.add(Object::Metaxploit(MetaxploitRef {
                        access,
                        version: library.version,
                    })))
                }
                LibraryClass::Crypto => {
                    IncludedLibrary::Crypto(objects.add(Object::Crypto(access)))
                }
                LibraryClass::AptClient => {
                    IncludedLibrary::AptClient(objects.add(Object::AptClient(access)))
                }
                LibraryClass::Blockchain => {
                    IncludedLibrary::Blockchain(objects.add(Object::Blockchain(access)))
                }
                LibraryClass::SmartAppliance => {
                    IncludedLibrary::SmartAppliance(objects.add(Object::SmartAppliance(access)))
                }
                LibraryClass::TrafficNet => {
                    IncludedLibrary::TrafficNet(objects.add(Object::TrafficNet(access)))
                }
                LibraryClass::Service(service) => IncludedLibrary::Service(
                    objects.add(Object::Service(ServiceRef { access, service })),
                ),
            })
        },
    );
    connection.read(
        channel,
        async |world, objects, message: PlayerGetShell<'static>, caller| {
            let session = objects.session(caller.session)?;
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
                    .check_login(access.computer, &message.user, &message.password)
                    .await?
                {
                    Login::Valid => ShellLogin::Shell(objects.add(Object::Shell(access))),
                    Login::InvalidCredentials => ShellLogin::Null,
                    Login::AccountInactive => ShellLogin::AccountInactive,
                },
            )
        },
    );
    connection.read(
        channel,
        async |world, objects, _: PlayerUserMailAddress, caller| {
            let computer = objects.session(caller.session)?.computer;

            Ok(world
                .player_account(computer, PlayerAccount::MailAddress)
                .await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, _: PlayerUserBankNumber, caller| {
            let computer = objects.session(caller.session)?.computer;

            Ok(world
                .player_account(computer, PlayerAccount::BankNumber)
                .await?)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: PlayerMailLogin<'static>, caller| {
            let access = objects.session(caller.session)?.clone();

            Ok(
                match world
                    .mail_login_from(
                        access.computer,
                        &caller.terminal_computers,
                        &message.address,
                        &message.password,
                    )
                    .await?
                {
                    Some(Login::Valid) => {
                        let emails = world
                            .mail_account(&message.address)
                            .await?
                            .map_or_else(Vec::new, |account| account.inbox);

                        ObjectOutcome::Object(objects.add(Object::MetaMail(MailRef {
                            access,
                            address: message.address.to_string(),
                            emails,
                        })))
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
    connection.read(
        channel,
        async |world, objects, message: PlayerGetCtf<'static>, caller| {
            let computer = objects.session(caller.session)?.computer;

            Ok(world
                .ctf_login(&message.event, &message.user, &message.password)
                .await?
                .then(|| {
                    objects.add(Object::Ctf(CtfRef {
                        computer,
                        event: message.event.to_string(),
                    }))
                }))
        },
    );
    connection.change(
        channel,
        async |world, objects, message: PlayerResetCtfPassword<'static>, caller| {
            let computer = objects.session(caller.session)?.computer;

            Ok(
                match world
                    .change_ctf_password(computer, &message.password)
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(CtfPasswordFailure::NoAccount) => {
                        Outcome::Failure(Failure::Ctf(CtfFailure::NoAccount))
                    }
                    Err(CtfPasswordFailure::Refusal(refusal)) => {
                        Outcome::Failure(Failure::Refusal(refusal.into()))
                    }
                },
            )
        },
    );
}
