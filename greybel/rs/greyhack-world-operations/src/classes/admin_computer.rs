//! Single-computer `admin.so` operations (`adminComputer`), addressed by public and LAN IP. Includes the
//! game's UI/service SQL, CCTV, and ADB channels.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::ServiceId;
use greyhack_world::{
    CctvFailure, ComputerId, NetCard, Record, RecordFailure, SmartApplianceSettings, World,
};

use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{Access, Object};
use crate::protocol::{
    AdminAccessCctv, AdminAddNetCard, AdminAddNpc, AdminAddSmartAppliance, AdminFailure,
    AdminInstallSmartphoneAdb, AdminLogin, AdminNpcLogin, AdminServiceRecords,
    AdminSetLibraryVersion, AdminSetPortLibraryVersion, AdminShell, AdminUpdateServiceRecord,
    CctvAccess, Failure, GameServer, Outcome, SmartApplianceOptions,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.change(
        channel,
        async |world, _, message: AdminAddSmartAppliance<'static>, _| {
            add_smart_appliance(
                world,
                &message.public_ip,
                &message.lan_ip,
                &message.appliance,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminAddNpc<'static>, _| {
            add_npc(
                world,
                &message.public_ip,
                &message.lan_ip,
                &message.user,
                message.delay,
                &message.name,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: AdminShell<'static>, _| {
            Ok(
                shell(world, &message.public_ip, &message.lan_ip, &message.user)
                    .await?
                    .map(|computer| {
                        objects.add(Object::Shell(Access {
                            computer,
                            user: message.user.to_string(),
                            ftp: false,
                            prev_ip: String::new(),
                            bounces: 0,
                        }))
                    }),
            )
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminLogin<'static>, _| {
            login(world, &message.public_ip, &message.lan_ip, &message.user).await
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminNpcLogin<'static>, _| {
            // Apply social engineering (`Persona.ProcesaIngenieraSocial`): the NPC logs in through SSH/FTP,
            // exposing credentials to sniffers along the path.
            let Some(computer) = world.computer_at(&message.address, &message.lan_ip).await? else {
                return Ok(false);
            };

            world.npc_social_login(computer, &message.user).await?;

            Ok(true)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminAddNetCard<'static>, _| {
            let Some(computer) = world
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };
            let card = NetCard::new(message.wifi, message.monitor_support);

            Ok(world.add_net_card(computer, &card).await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminSetLibraryVersion<'static>, _| {
            let Some(computer) = world
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            Ok(world
                .set_library_version(computer, &message.path, &message.version)
                .await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminSetPortLibraryVersion<'static>, _| {
            let Some(computer) = world
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            Ok(world
                .set_port_library_version(computer, message.port, &message.version)
                .await?)
        },
    );
    connection.read(
        channel,
        async |world, _, message: AdminServiceRecords<'static>, _| {
            service_records(world, &message).await
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminUpdateServiceRecord<'static>, _| {
            update_service_record(world, &message).await
        },
    );
    connection.read(
        channel,
        async |world, _, message: AdminAccessCctv<'static>, _| access_cctv(world, &message).await,
    );
    connection.change(
        channel,
        async |world, _, message: AdminInstallSmartphoneAdb<'static>, _| {
            let Some(computer) = world
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            Ok(world.install_smartphone_adb(computer).await?)
        },
    );
}

/// Enables smart-appliance settings on the computer at `lan_ip` behind `public_ip`.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn add_smart_appliance(
    world: &mut World,
    public_ip: &str,
    lan_ip: &str,
    appliance: &SmartApplianceOptions,
) -> Result<bool> {
    let Some(computer) = world.computer_at(public_ip, lan_ip).await? else {
        return Ok(false);
    };
    let mut settings =
        SmartApplianceSettings::new(&appliance.model, appliance.power, appliance.temperature);

    settings.override_enabled = appliance.override_enabled;

    Ok(world.enable_smart_appliance(computer, &settings).await?)
}

/// Creates an offline NPC scheduled to log in after `delay` seconds and returns its ID.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn add_npc(
    world: &mut World,
    public_ip: &str,
    lan_ip: &str,
    user: &str,
    delay: u64,
    name: &str,
) -> Result<Option<u32>> {
    let next_action_at = world.time_seconds().saturating_add(delay);
    let Some(computer) = world.computer_at(public_ip, lan_ip).await? else {
        return Ok(None);
    };

    Ok(world
        .add_npc(computer, user, name, next_action_at)
        .await?
        .map(|id| id.number()))
}

/// Finds the computer at `lan_ip` behind `public_ip` only if it contains `user`.
///
/// # Errors
///
/// Returns errors from database searches.
pub async fn shell(
    world: &World,
    public_ip: &str,
    lan_ip: &str,
    user: &str,
) -> Result<Option<ComputerId>> {
    let Some(id) = world.computer_at(public_ip, lan_ip).await? else {
        return Ok(None);
    };
    let has_user = world.user(id, user).await?.is_some();

    Ok(has_user.then_some(id))
}

/// Opens a session for `user` on the computer at `lan_ip` behind `public_ip` unless missing or already open.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn login(world: &mut World, public_ip: &str, lan_ip: &str, user: &str) -> Result<bool> {
    let Some(id) = world.computer_at(public_ip, lan_ip).await? else {
        return Ok(false);
    };

    Ok(world.open_new_session(id, user).await?)
}

/// Returns service records as UI-viewer JSON (`FileHelperServer.LeerFichaServerRpc`).
async fn service_records(
    world: &World,
    message: &AdminServiceRecords<'_>,
) -> Result<Option<String>> {
    let Some(computer) = world
        .computer_at(&message.public_ip, &message.lan_ip)
        .await?
    else {
        return Ok(None);
    };
    let Some(service) = ServiceId::from_name(&message.service) else {
        return Ok(None);
    };

    Ok(world
        .service_records(computer, service)
        .await?
        .map(|records| records.to_json()))
}

/// Applies a UI-viewer record update and returns any failure (`FileHelperServer.UpdateFichaServerRpc`).
async fn update_service_record(
    world: &mut World,
    message: &AdminUpdateServiceRecord<'_>,
) -> Result<Outcome> {
    let failure = |failure| Ok(Outcome::Failure(Failure::Admin(failure)));
    let Some(computer) = world
        .computer_at(&message.public_ip, &message.lan_ip)
        .await?
    else {
        return failure(AdminFailure::MissingRecord);
    };
    let Some(service) = ServiceId::from_name(&message.service) else {
        return failure(AdminFailure::CorruptedRecords);
    };
    let Some(record) = Record::parse_for(service, &message.record) else {
        return failure(AdminFailure::CorruptedRecords);
    };

    match world
        .update_service_record(computer, service, record)
        .await?
    {
        Ok(()) => Ok(Outcome::True),
        Err(RecordFailure::Unreadable) => failure(AdminFailure::CorruptedRecords),
        Err(RecordFailure::MissingEntry) => failure(AdminFailure::MissingRecord),
    }
}

/// Handles browser CCTV access (`NetworkHelperServer.AccessWebServerRpc`).
async fn access_cctv(world: &World, message: &AdminAccessCctv<'_>) -> Result<CctvAccess> {
    let Some(computer) = world
        .computer_at(&message.public_ip, &message.lan_ip)
        .await?
    else {
        return Ok(CctvAccess::Failure(Failure::Admin(
            AdminFailure::WrongCameraPassword,
        )));
    };

    Ok(
        match world
            .access_cctv(computer, message.password.as_deref())
            .await?
        {
            Ok(None) => CctvAccess::Prompt,
            Ok(Some(feed)) => CctvAccess::Data(feed.to_json()),
            Err(CctvFailure::WrongPassword) => {
                CctvAccess::Failure(Failure::Admin(AdminFailure::WrongCameraPassword))
            }
            Err(CctvFailure::NoExternalAccess) => {
                CctvAccess::Failure(Failure::Admin(AdminFailure::CamerasDisabled))
            }
        },
    )
}
