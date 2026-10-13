//! Single-network `admin.so` operations (`adminNetwork`) addressed by public IP.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::ROOT_USER;
use greyhack_world::{ComputerId, JobRole, LibraryReportFailure, PhoneAlarm, World};

use super::admin::Creation;
use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{Access, Object};
use crate::protocol::{
    AdminAddComputer, AdminDisableNetwork, AdminFailure, AdminFireNpc, AdminFireRentalAdmin,
    AdminHireNpc, AdminHireRentalAdmin, AdminPhoneAlarm, AdminRemoveNetwork,
    AdminReportLibraryUpdate, AdminSetAdminRestore, AdminSetNetSpeed, AdminSetPhoneAlarm,
    ComputerCreation, ComputerOptions, Failure, GameServer, JobFailure, Outcome, PhoneAlarmConfig,
    ScriptFailure,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.change(
        channel,
        async |world, objects, message: AdminAddComputer<'static>, _| {
            Ok(
                match add_computer(world, &message.public_ip, message.computer).await? {
                    Creation::Object(computer) => {
                        ComputerCreation::Computer(objects.add(Object::Computer(Access {
                            computer,
                            user: ROOT_USER.to_owned(),
                            ftp: false,
                            prev_ip: String::new(),
                            bounces: 0,
                        })))
                    }
                    Creation::Failure(failure) => {
                        ComputerCreation::Failure(Failure::Admin(failure))
                    }
                },
            )
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminFireNpc<'static>, _| {
            Ok(job_outcome(
                world.fire_npc(&message.public_ip, &message.user).await?,
                JobFailure::from,
            ))
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminHireNpc<'static>, _| {
            let Some(role) = JobRole::ALL
                .into_iter()
                .find(|role| role.game_name() == message.job_role)
            else {
                return Ok(Outcome::Failure(Failure::Job(JobFailure::InvalidJob)));
            };

            Ok(job_outcome(
                world.hire_npc(&message.public_ip, role).await?,
                JobFailure::from,
            ))
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminHireRentalAdmin<'static>, _| {
            Ok(job_outcome(
                world.hire_rental_admin(&message.public_ip).await?,
                JobFailure::from,
            ))
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminFireRentalAdmin<'static>, _| {
            Ok(job_outcome(
                world.fire_rental_admin(&message.public_ip).await?,
                JobFailure::from,
            ))
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminRemoveNetwork<'static>, _| {
            Ok(world.remove_network(&message.public_ip).await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminDisableNetwork<'static>, _| {
            Ok(world
                .disable_network(&message.public_ip, message.disabled)
                .await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminSetNetSpeed<'static>, _| {
            Ok(world
                .set_net_speed(&message.public_ip, message.net_speed)
                .await?)
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminSetPhoneAlarm<'static>, _| match world
            .set_phone_alarm(
                &message.public_ip,
                &message.user,
                message.hour,
                message.minute,
                message.delete,
            )
            .await?
        {
            None => Ok(None),
            Some(Ok(alarm)) => Ok(Some(alarm_config(alarm))),
            Some(Err(failure)) => {
                Err(ScriptFailure::Refused(Failure::Alarm(failure.into())).into())
            }
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminPhoneAlarm<'static>, _| {
            Ok(world
                .phone_alarm(&message.public_ip, &message.user)
                .await?
                .map(alarm_config))
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminReportLibraryUpdate<'static>, _| {
            Ok(
                match world
                    .report_library_update(&message.public_ip, &message.library)
                    .await?
                {
                    None => Outcome::Null,
                    Some(Ok(())) => Outcome::True,
                    Some(Err(LibraryReportFailure::UnknownLibrary)) => Outcome::False,
                    Some(Err(LibraryReportFailure::AdminRestoreOff)) => {
                        Outcome::Failure(Failure::Admin(AdminFailure::LibraryUpdateSkipped))
                    }
                },
            )
        },
    );
    connection.change(
        channel,
        async |world, _, message: AdminSetAdminRestore<'static>, _| {
            Ok(
                match world
                    .set_admin_restore(&message.guild, &message.public_ip, message.enabled)
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(failure) => Outcome::Failure(Failure::Admin(failure.into())),
                },
            )
        },
    );
}

fn alarm_config(alarm: PhoneAlarm) -> PhoneAlarmConfig {
    PhoneAlarmConfig {
        hour: alarm.config.hour,
        minute: alarm.config.minute,
        modified: alarm.config.is_modified,
        deleted: alarm.config.is_deleted,
        late_strikes: alarm.late_strikes,
        fired: alarm.fired,
    }
}

/// Creates a computer on the network at `public_ip`.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn add_computer(
    world: &mut World,
    public_ip: &str,
    computer: ComputerOptions,
) -> Result<Creation<ComputerId>> {
    let Some(router) = world.router_at(public_ip).await?.map(|router| router.id) else {
        return Ok(Creation::Failure(AdminFailure::UnknownNetwork(
            public_ip.to_owned(),
        )));
    };
    let hardware = computer.mining_hardware;
    let options = greyhack_world::ComputerOptions {
        name: computer.name,
        lan_ip: computer.lan_ip,
        root_password: computer.root_password,
        users: computer.users.map(|users| {
            users
                .into_iter()
                .map(|user| (user.name, user.password))
                .collect()
        }),
        services: computer.services,
        is_switch: computer.is_switch,
        mining_hardware: greyhack_world::MiningHardware {
            cpu_speed: hardware.cpu_speed,
            cpu_cores: hardware.cpu_cores,
            ram_model: hardware.ram_model,
            gpu_hashrate: hardware.gpu_hashrate,
        },
    };

    Ok(world.add_computer_to(router, &options).await?.map_or_else(
        || Creation::Failure(AdminFailure::LanAddressTaken),
        Creation::Object,
    ))
}

/// Converts a job result to null for a missing network, true on success, or the job failure.
fn job_outcome<T, F>(
    result: Option<Result<T, F>>,
    failure: impl FnOnce(F) -> JobFailure,
) -> Outcome {
    match result {
        None => Outcome::Null,
        Some(Ok(_)) => Outcome::True,
        Some(Err(refusal)) => Outcome::Failure(Failure::Job(failure(refusal))),
    }
}
