//! Repository and package-installation operations (`AptClientIntrinsics`).

use greyhack_world_programs::protocol::{
    AptClientAddRepo, AptClientCheckUpgrade, AptClientDelRepo, AptClientInstall, AptClientSearch,
    AptClientShow, AptClientUpdate, AptUpdate, Failure, Outcome, PackageEntry, PackageList,
    PrintLine,
};
use greyhack_world_router::RouteTable;

use crate::objects::ObjectTableExt;
use crate::routes::protocol::IntoProtocol;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    register_apt_client_repositories::<W>(routes);
    register_apt_client_queries::<W>(routes);
}

fn register_apt_client_repositories<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AptClientAddRepo<'static>| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .apt_client(message.apt_client)?;

            Ok(apt_outcome(
                world
                    .module::<crate::ServicesModule>()
                    .apt
                    .add_apt_repository(access.computer, &message.repository, message.port)
                    .await?,
            ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AptClientDelRepo<'static>| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .apt_client(message.apt_client)?;

            Ok(apt_outcome(
                world
                    .module::<crate::ServicesModule>()
                    .apt
                    .remove_apt_repository(access.computer, &message.repository)
                    .await?,
            ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AptClientInstall<'static>| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .apt_client(message.apt_client)?;
            let terminal_computers = &ctx.terminal_computers(call);

            Ok(apt_outcome(
                world
                    .module::<crate::ServicesModule>()
                    .apt
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
}

fn register_apt_client_queries<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AptClientSearch<'static>| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .apt_client(message.apt_client)?;
            let terminal_computers = &ctx.terminal_computers(call);

            Ok(package_list(
                world
                    .module::<crate::ServicesModule>()
                    .apt
                    .search_apt_packages(access.computer, &message.term, terminal_computers)
                    .await?,
            ))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AptClientShow<'static>| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .apt_client(message.apt_client)?;
            let terminal_computers = &ctx.terminal_computers(call);

            Ok(package_list(
                world
                    .module::<crate::ServicesModule>()
                    .apt
                    .show_apt_repository(access.computer, &message.repository, terminal_computers)
                    .await?,
            ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AptClientUpdate| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .apt_client(message.apt_client)?;
            let terminal_computers = &ctx.terminal_computers(call);

            let lines = world
                .module::<crate::ServicesModule>()
                .apt
                .update_apt_packages(access.computer, terminal_computers)
                .await?;

            let result = lines
                .iter()
                .find_map(|line| match line {
                    crate::AptUpdateLine::UnknownRepository(source) => {
                        Some(format!("{source} repository not found"))
                    }
                    crate::AptUpdateLine::MalformedSources => Some("0".to_owned()),
                    _ => None,
                })
                .unwrap_or_default();

            for line in lines {
                if !matches!(line, crate::AptUpdateLine::UnknownRepository(_)) {
                    ctx.print(call, PrintLine::AptUpdate(line.into_protocol()));
                }
            }

            Ok(AptUpdate { result })
        },
    );
    // A HackShop's libraries are brought up to date on the way (`PlayerUtils.UpdateHackshopLibs`), which changes it.
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: AptClientCheckUpgrade<'static>| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .apt_client(message.apt_client)?;
            let terminal_computers = &ctx.terminal_computers(call);

            let report = world
                .module::<crate::ServicesModule>()
                .apt
                .check_apt_upgrade(access.computer, &message.path, terminal_computers)
                .await?;

            for _ in 0..report.malformed_repositories {
                ctx.print(
                    call,
                    PrintLine::AptUpdate(crate::AptUpdateLine::MalformedConfig.into_protocol()),
                );
            }

            Ok(match report.result {
                Ok(newer) => newer.into_protocol(),
                Err(failure) => Outcome::Failure(Failure::Apt(failure.into_protocol())),
            })
        },
    );
}

/// Converts `result` to `true` on success or its apt failure.
fn apt_outcome(result: Result<(), crate::AptFailure>) -> Outcome {
    result.map_or_else(
        |failure| Outcome::Failure(Failure::Apt(failure.into_protocol())),
        |()| Outcome::True,
    )
}

/// Converts package `result` to a list or apt failure.
fn package_list(result: Result<Vec<crate::PackageEntry>, crate::AptFailure>) -> PackageList {
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
        Err(failure) => PackageList::Failure(Failure::Apt(failure.into_protocol())),
    }
}
