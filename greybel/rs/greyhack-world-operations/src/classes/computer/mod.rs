//! Operations on computer objects (`ComputerIntrinsics`).

mod files;
mod network;
mod processes;
mod users;

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::WifiRequest;

pub(crate) use self::files::creation_outcome;
use self::files::{create_folder, file, touch};
use self::network::{
    active_net_card, connect_ethernet, connect_wifi, get_ports, network_devices, wifi_networks,
};
use self::processes::{close_program, reboot, show_procs};
use self::users::{change_password, create_group, create_user, delete_group, delete_user, groups};
use super::computer;
use crate::connection::Connection;
use crate::connection::objects::{FileRef, Object};
use crate::protocol::{
    ComputerActiveNetCard, ComputerChangePassword, ComputerCloseProgram, ComputerConnectEthernet,
    ComputerConnectWifi, ComputerCreateFolder, ComputerCreateGroup, ComputerCreateUser,
    ComputerDeleteGroup, ComputerDeleteUser, ComputerFile, ComputerGetName, ComputerGetPorts,
    ComputerGroups, ComputerIsNetworkActive, ComputerLocalIp, ComputerNetworkDevices,
    ComputerNetworkGateway, ComputerPublicIp, ComputerReboot, ComputerShowProcs, ComputerTouch,
    ComputerWifiNetworks, GameServer,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: ComputerGetPorts, _| {
            get_ports(world, objects.computer(message.computer)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerFile<'static>, _| {
            let access = objects.computer(message.computer)?.clone();

            Ok(file(world, &access, &message.path).await?.map(|node| {
                objects.add(Object::File(FileRef {
                    access,
                    node: node.id,
                    path: node.path,
                }))
            }))
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerTouch<'static>, _| {
            touch(
                world,
                objects.computer(message.computer)?,
                &message.folder,
                &message.name,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerCreateFolder<'static>, _| {
            create_folder(
                world,
                objects.computer(message.computer)?,
                &message.folder,
                &message.name,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerIsNetworkActive, caller| {
            Ok(world
                .is_network_active(
                    objects.computer(message.computer)?.computer,
                    &caller.terminal_computers,
                )
                .await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerGetName, _| {
            Ok(computer(world, objects.computer(message.computer)?)
                .await?
                .map(|computer| computer.name))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerActiveNetCard, _| {
            active_net_card(world, objects.computer(message.computer)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerNetworkGateway, _| {
            Ok(computer(world, objects.computer(message.computer)?)
                .await?
                .map(|computer| computer.gateway))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerLocalIp, _| {
            Ok(computer(world, objects.computer(message.computer)?)
                .await?
                .map(|computer| computer.local_ip))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerPublicIp, _| {
            Ok(world
                .public_ip(objects.computer(message.computer)?.computer)
                .await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerNetworkDevices, _| {
            network_devices(world, objects.computer(message.computer)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerShowProcs, _| {
            show_procs(world, objects.computer(message.computer)?).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerCloseProgram, caller| {
            close_program(
                world,
                objects.computer(message.computer)?,
                message.pid,
                caller.root_process,
                caller.terminal_command_process,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerChangePassword<'static>, _| {
            change_password(
                world,
                objects.computer(message.computer)?,
                &message.user,
                &message.password,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ComputerGroups<'static>, _| {
            groups(world, objects.computer(message.computer)?, &message.user).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerCreateGroup<'static>, _| {
            create_group(
                world,
                objects.computer(message.computer)?,
                &message.user,
                &message.group,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerDeleteGroup<'static>, _| {
            delete_group(
                world,
                objects.computer(message.computer)?,
                &message.user,
                &message.group,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerCreateUser<'static>, _| {
            create_user(
                world,
                objects.computer(message.computer)?,
                &message.user,
                &message.password,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerDeleteUser<'static>, _| {
            delete_user(
                world,
                objects.computer(message.computer)?,
                &message.user,
                message.remove_home,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerReboot, caller| {
            reboot(
                world,
                objects.computer(message.computer)?,
                &caller.terminal_computers,
                message.safe_mode,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerWifiNetworks<'static>, _| {
            wifi_networks(world, objects.computer(message.computer)?, &message.device).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ComputerConnectWifi<'static>, _| {
            connect_wifi(
                world,
                objects.computer(message.computer)?,
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
    connection.change(
        channel,
        async |world, objects, message: ComputerConnectEthernet, caller| {
            connect_ethernet(
                world,
                objects.computer(message.computer)?,
                &caller.terminal_computers,
                message.settings.as_ref(),
                message.wait,
            )
            .await
        },
    );
}
