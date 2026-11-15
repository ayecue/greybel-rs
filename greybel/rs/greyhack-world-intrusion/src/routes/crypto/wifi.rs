//! Monitor mode, capture, and key recovery (`CryptoIntrinsics.airmon`, `aireplay`, `aircrack`).

use greyhack_world_core::Access;
use greyhack_world_router::{Call, RouteContext};

use crate::routes::protocol::{
    Capture, Failure, IntoProtocol as _, KeyRecovery, Outcome, PrintLine,
};
use crate::routes::{OperationError, Result};

/// Sets monitor mode on `device` to `start`.
pub(super) async fn airmon(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    access: &Access,
    start: bool,
    device: &str,
) -> Result<Outcome> {
    Ok(
        match world
            .module::<crate::IntrusionModule>()
            .crypto
            .set_monitor_mode(access.computer, device, start)
            .await?
        {
            Ok(has_device) => has_device.into_protocol(),
            Err(failure) => Outcome::Failure(failure.into_protocol()),
        },
    )
}

/// Captures Wi-Fi ACKs for `bssid`/`essid` and writes `file.cap` under `folder`. Returns startup or save
/// failures.
pub(super) async fn aireplay(
    ctx: &RouteContext<
        '_,
        impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    >,
    call: Call,
    access: &Access,
    network: (&str, &str),
    max_acks: i32,
    folder: &str,
) -> Result<Capture> {
    let world = &*ctx.modules;

    Ok(
        match world
            .module::<crate::IntrusionModule>()
            .crypto
            .capture_wifi(access.computer, &access.user, network, max_acks, folder)
            .await?
        {
            None => Capture::Null,
            Some(Err(failure)) => Capture::Failure(failure.into_protocol()),
            Some(Ok(run)) => {
                let save_failure = run
                    .save_failure
                    .map(|failure| {
                        <Failure as greyhack_world_core::routes::conversions::TryProtocolFrom<
                                _,
                            >>::try_from(failure)
                            .map_err(OperationError::Script)
                    })
                    .transpose()?;

                ctx.print(call, PrintLine::CaptureHandshake { bssid: run.bssid });
                ctx.print(
                    call,
                    PrintLine::CaptureProgress {
                        packets: run.count.packets,
                        acks: run.count.acks,
                    },
                );

                if let Some(failure) = save_failure {
                    ctx.print(call, PrintLine::CaptureSaveFailure(failure));
                }

                Capture::Null
            }
        },
    )
}

/// Recovers a Wi-Fi password from `path`, returning capture or lookup failures.
pub(super) async fn aircrack(
    ctx: &RouteContext<
        '_,
        impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    >,
    call: Call,
    access: &Access,
    path: &str,
) -> Result<KeyRecovery> {
    let world = &*ctx.modules;

    Ok(
        match world
            .module::<crate::IntrusionModule>()
            .crypto
            .recover_wifi_password(access.computer, &access.user, path)
            .await?
        {
            None => KeyRecovery::Null,
            Some(Ok(password)) => KeyRecovery::Password(password),
            Some(Err(failure)) => {
                ctx.print(call, PrintLine::KeyRecoveryFailure(failure.into_protocol()));

                KeyRecovery::Null
            }
        },
    )
}
