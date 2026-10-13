//! Repository and package-installation operations (`AptClientIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};

use super::require_connection;
use crate::connection::Connection;
use crate::protocol::{
    AptClientAddRepo, AptClientCheckUpgrade, AptClientDelRepo, AptClientInstall, AptClientSearch,
    AptClientShow, AptClientUpdate, AptUpdate, Failure, GameServer, Outcome, PackageEntry,
    PackageList,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.change(
        channel,
        async |world, objects, message: AptClientAddRepo<'static>, _| {
            let access = objects.apt_client(message.apt_client)?;

            Ok(apt_outcome(
                world
                    .add_apt_repository(access.computer, &message.repository, message.port)
                    .await?,
            ))
        },
    );
    connection.change(
        channel,
        async |world, objects, message: AptClientDelRepo<'static>, _| {
            let access = objects.apt_client(message.apt_client)?;

            Ok(apt_outcome(
                world
                    .remove_apt_repository(access.computer, &message.repository)
                    .await?,
            ))
        },
    );
    connection.change(
        channel,
        async |world, objects, message: AptClientInstall<'static>, caller| {
            let access = objects.apt_client(message.apt_client)?;
            let terminal_computers = &caller.terminal_computers;

            require_connection(world, terminal_computers).await?;

            Ok(apt_outcome(
                world
                    .install_apt_package(
                        access.computer,
                        &access.user,
                        &message.package,
                        &message.folder,
                        terminal_computers,
                    )
                    .await?,
            ))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: AptClientSearch<'static>, caller| {
            let access = objects.apt_client(message.apt_client)?;
            let terminal_computers = &caller.terminal_computers;

            Ok(package_list(
                world
                    .search_apt_packages(access.computer, &message.term, terminal_computers)
                    .await?,
            ))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: AptClientShow<'static>, caller| {
            let access = objects.apt_client(message.apt_client)?;
            let terminal_computers = &caller.terminal_computers;

            Ok(package_list(
                world
                    .show_apt_repository(access.computer, &message.repository, terminal_computers)
                    .await?,
            ))
        },
    );
    connection.change(
        channel,
        async |world, objects, message: AptClientUpdate, caller| {
            let access = objects.apt_client(message.apt_client)?;
            let terminal_computers = &caller.terminal_computers;

            require_connection(world, terminal_computers).await?;

            Ok(AptUpdate {
                lines: world
                    .update_apt_packages(access.computer, terminal_computers)
                    .await?
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            })
        },
    );
    // A HackShop's libraries are brought up to date on the way (`PlayerUtils.UpdateHackshopLibs`), which changes it.
    connection.change(
        channel,
        async |world, objects, message: AptClientCheckUpgrade<'static>, caller| {
            let access = objects.apt_client(message.apt_client)?;
            let terminal_computers = &caller.terminal_computers;

            if let Err(failure) = world.validate_apt_cache(access.computer).await? {
                return Ok(Outcome::Failure(Failure::Apt(failure.into())));
            }

            Ok(
                match world
                    .check_apt_upgrade(access.computer, &message.path, terminal_computers)
                    .await?
                {
                    None => Outcome::Null,
                    Some(value) => Outcome::from(value),
                },
            )
        },
    );
}

/// Converts `result` to `true` on success or its apt failure.
fn apt_outcome(result: Result<(), greyhack_world::AptFailure>) -> Outcome {
    Outcome::from(result.map_err(|failure| Failure::Apt(failure.into())))
}

/// Converts package `result` to a list or apt failure.
fn package_list(
    result: Result<Vec<greyhack_world::PackageEntry>, greyhack_world::AptFailure>,
) -> PackageList {
    match result {
        Ok(packages) => PackageList::Packages(
            packages
                .into_iter()
                .map(|package| PackageEntry {
                    name: package.name,
                    description: package.description,
                })
                .collect(),
        ),
        Err(failure) => PackageList::Failure(Failure::Apt(failure.into())),
    }
}
