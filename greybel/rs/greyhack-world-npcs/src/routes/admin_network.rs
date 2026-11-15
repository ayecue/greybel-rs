//! NPC hiring, firing, rented administrators, and alarms of the `adminNetwork` object.

use greyhack_behaviour::JobRole;
use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{
    AdminFireNpc, AdminFireRentalAdmin, AdminHireNpc, AdminHireRentalAdmin, AdminPhoneAlarm,
    AdminSetPhoneAlarm, Failure, JobFailure, Outcome, PhoneAlarmConfig, ScriptFailure,
};

use crate::PhoneAlarm;
use crate::routes::protocol::IntoProtocol;
use crate::routes::{OperationError, Result};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminFireNpc<'static>| {
            Ok(job_outcome(
                ctx.modules
                    .module::<crate::NpcModule>()
                    .jobs
                    .fire_npc(&message.public_ip, &message.user)
                    .await?,
                IntoProtocol::into_protocol,
            ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminHireNpc<'static>| {
            let Some(role) = JobRole::ALL
                .into_iter()
                .find(|role| role.game_name() == message.job_role)
            else {
                return Ok(Outcome::Failure(Failure::Job(JobFailure::InvalidJob)));
            };

            Ok(job_outcome(
                ctx.modules
                    .module::<crate::NpcModule>()
                    .jobs
                    .hire_npc(&message.public_ip, role)
                    .await?,
                IntoProtocol::into_protocol,
            ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminHireRentalAdmin<'static>| {
            Ok(job_outcome(
                ctx.modules
                    .module::<crate::NpcModule>()
                    .jobs
                    .hire_rental_admin(&message.public_ip)
                    .await?,
                IntoProtocol::into_protocol,
            ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminFireRentalAdmin<'static>| {
            Ok(job_outcome(
                ctx.modules
                    .module::<crate::NpcModule>()
                    .jobs
                    .fire_rental_admin(&message.public_ip)
                    .await?,
                IntoProtocol::into_protocol,
            ))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminSetPhoneAlarm<'static>| {
            match ctx
                .modules
                .module::<crate::NpcModule>()
                .schedules
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
                Some(Err(failure)) => Err(OperationError::Script(ScriptFailure::Refused(
                    Failure::Alarm(failure.into_protocol()),
                ))),
            }
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminPhoneAlarm<'static>| {
            Ok(ctx
                .modules
                .module::<crate::NpcModule>()
                .schedules
                .phone_alarm(&message.public_ip, &message.user)
                .await?
                .map(alarm_config))
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
