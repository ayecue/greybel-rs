//! Operations on computer objects (`ComputerIntrinsics`).

mod files;
mod network;
mod processes;
mod users;

use greyhack_world_router::protocol::{
    ComputerActiveNetCard, ComputerChangePassword, ComputerCloseProgram, ComputerConnectEthernet,
    ComputerConnectWifi, ComputerCreateFolder, ComputerCreateGroup, ComputerCreateUser,
    ComputerDeleteGroup, ComputerDeleteUser, ComputerFile, ComputerGetName, ComputerGetPorts,
    ComputerGroups, ComputerIsNetworkActive, ComputerLocalIp, ComputerNetworkDevices,
    ComputerNetworkGateway, ComputerPublicIp, ComputerReboot, ComputerShowProcs, ComputerTouch,
    ComputerWifiNetworks, Outcome,
};
use greyhack_world_router::{RouteContext, RouteTable};

pub use self::files::creation_outcome;
use self::files::{create_folder, file, touch};
use self::network::{
    active_net_card, connect_ethernet, connect_wifi, get_ports, network_devices, wifi_networks,
};
use self::processes::{close_program, reboot, show_procs};
use self::users::{change_password, create_group, create_user, delete_group, delete_user, groups};
use super::computer;
use crate::WifiRequest;
use crate::objects::{FileHandle, FileRef, ObjectTableExt};

pub(crate) fn register<W: crate::CoreContext + 'static>(routes: &RouteTable) {
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerGetPorts| {
            let world = &*ctx.modules;

            get_ports(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerFile<'static>| {
            let world = &*ctx.modules;
            let access = ctx
                .trees
                .objects(call.tree)
                .computer(message.computer)?
                .clone();

            Ok(file(world, &access, &message.path).await?.map(|node| {
                ctx.trees.objects(call.tree).insert(FileHandle(FileRef {
                    access,
                    node: node.id,
                    path: node.path,
                }))
            }))
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerTouch<'static>| {
            let world = &*ctx.modules;

            touch(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.folder,
                &message.name,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerCreateFolder<'static>| {
            let world = &*ctx.modules;

            create_folder(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.folder,
                &message.name,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerIsNetworkActive| {
            let world = &*ctx.modules;

            Ok(world
                .core()
                .networks
                .reach
                .is_network_active(
                    ctx.trees
                        .objects(call.tree)
                        .computer(message.computer)?
                        .computer,
                    &ctx.terminal_computers(call),
                )
                .await?)
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerGetName| {
            let world = &*ctx.modules;

            Ok(computer(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
            )
            .await?
            .map(|computer| computer.name))
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerActiveNetCard| {
            let world = &*ctx.modules;

            active_net_card(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerNetworkGateway| {
            let world = &*ctx.modules;

            Ok(computer(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
            )
            .await?
            .map(|computer| computer.gateway))
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerLocalIp| {
            let world = &*ctx.modules;

            Ok(computer(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
            )
            .await?
            .map(|computer| computer.local_ip))
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerPublicIp| {
            let world = &*ctx.modules;

            Ok(world
                .core()
                .networks
                .topology
                .public_ip(
                    ctx.trees
                        .objects(call.tree)
                        .computer(message.computer)?
                        .computer,
                )
                .await?)
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerNetworkDevices| {
            let world = &*ctx.modules;

            network_devices(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerShowProcs| {
            let world = &*ctx.modules;

            show_procs(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerCloseProgram| {
            let world = &*ctx.modules;

            close_program(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                message.pid,
                ctx.root_process(call),
                ctx.command_process(call),
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerChangePassword<'static>| {
            let world = &*ctx.modules;

            change_password(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.user,
                &message.password,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerGroups<'static>| {
            let world = &*ctx.modules;

            groups(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.user,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerCreateGroup<'static>| {
            let world = &*ctx.modules;

            create_group(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.user,
                &message.group,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerDeleteGroup<'static>| {
            let world = &*ctx.modules;

            delete_group(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.user,
                &message.group,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerCreateUser<'static>| {
            let world = &*ctx.modules;

            create_user(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.user,
                &message.password,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerDeleteUser<'static>| {
            let world = &*ctx.modules;

            delete_user(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.user,
                message.remove_home,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerReboot| {
            let world = &*ctx.modules;
            let access = &ctx.trees.objects(call.tree).computer(message.computer)?;
            let outcome = reboot(
                world,
                access,
                &ctx.terminal_computers(call),
                message.safe_mode,
            )
            .await?;

            // `ComputerIntrinsics.reboot`: a remote computer that is the one the terminal runs on makes the
            // bash exit.
            if matches!(outcome, Outcome::True)
                && access.computer != world.world_ref().player()
                && access.computer == ctx.last_shell(call, world.world_ref().player())
            {
                ctx.reboot_terminal(call);
            }

            Ok(outcome)
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerWifiNetworks<'static>| {
            let world = &*ctx.modules;

            wifi_networks(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &message.device,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerConnectWifi<'static>| {
            let world = &*ctx.modules;

            connect_wifi(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &WifiRequest {
                    device: message.device.as_deref(),
                    bssid: message.bssid.as_deref(),
                    essid: message.essid.as_deref(),
                    password: message.password.as_deref(),
                },
                message.wait,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: ComputerConnectEthernet| {
            let world = &*ctx.modules;

            connect_ethernet(
                world,
                &ctx.trees.objects(call.tree).computer(message.computer)?,
                &ctx.terminal_computers(call),
                message.settings.as_ref(),
                message.wait,
            )
            .await
        },
    );
}
