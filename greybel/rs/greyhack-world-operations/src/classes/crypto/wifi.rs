//! Monitor mode, capture, and key recovery (`CryptoIntrinsics.airmon`, `aireplay`, `aircrack`).

use greyhack_world::World;

use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{Capture, CaptureProgress, Failure, KeyRecovery, Outcome};

/// Sets monitor mode on `device` to `start`.
pub(super) async fn airmon(
    world: &mut World,
    access: &Access,
    start: bool,
    device: &str,
) -> Result<Outcome> {
    Ok(
        match world
            .set_monitor_mode(access.computer, device, start)
            .await?
        {
            Ok(has_device) => has_device.into(),
            Err(failure) => Outcome::Failure(failure.into()),
        },
    )
}

/// Captures Wi-Fi ACKs for `bssid`/`essid` and writes `file.cap` under `folder`. Returns startup or save
/// failures.
pub(super) async fn aireplay(
    world: &mut World,
    access: &Access,
    network: (&str, &str),
    max_acks: i32,
    folder: &str,
) -> Result<Capture> {
    Ok(
        match world
            .capture_wifi(access.computer, &access.user, network, max_acks, folder)
            .await?
        {
            None => Capture::Null,
            Some(Err(failure)) => Capture::Failure(failure.into()),
            Some(Ok(run)) => {
                let progress = CaptureProgress {
                    bssid: run.bssid,
                    packets: run.count.packets,
                    acks: run.count.acks,
                };

                match run.save_failure {
                    None => Capture::Captured(progress),
                    Some(failure) => Capture::Unsaved {
                        progress,
                        failure: Failure::try_from(failure)?,
                    },
                }
            }
        },
    )
}

/// Recovers a Wi-Fi password from `path`, returning capture or lookup failures.
pub(super) async fn aircrack(world: &World, access: &Access, path: &str) -> Result<KeyRecovery> {
    Ok(
        match world
            .recover_wifi_password(access.computer, &access.user, path)
            .await?
        {
            None => KeyRecovery::Null,
            Some(Ok(password)) => KeyRecovery::Password(password),
            Some(Err(failure)) => KeyRecovery::Failure(failure.into()),
        },
    )
}
