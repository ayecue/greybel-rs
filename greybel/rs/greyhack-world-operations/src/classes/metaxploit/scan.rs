//! Library scans (`MetaExploitIntrinsics.scan`, `scan_address`).

use greyhack_world::World;

use crate::Result;
use crate::connection::objects::MetaLibRef;
use crate::protocol::{LibraryScan, ScriptFailure, ZoneScan};

/// Lists weak addresses supported by metaxploit `version` and flags unscanned zones that may contain
/// weaknesses.
pub(super) async fn scan(
    world: &World,
    meta_lib: &MetaLibRef,
    version: &str,
) -> Result<Option<LibraryScan>> {
    let Some(library) = crate::classes::meta_lib::library(world, meta_lib, Clone::clone).await?
    else {
        return Ok(None);
    };

    Ok(world
        .scan_library(&library, version)
        .await?
        .map(|scan| LibraryScan {
            addresses: scan.addresses,
            hidden_zones: scan.hidden_zones,
        }))
}

/// Scans `address` without a metaxploit helper; the game dereferences the helper only when weaknesses exist.
pub(super) async fn scan_address_without_metaxploit(
    world: &World,
    meta_lib: &MetaLibRef,
    address: &str,
) -> Result<Option<ZoneScan>> {
    let needs_helper = crate::classes::meta_lib::library(world, meta_lib, |library| {
        library.has_weaknesses_at(address)
    })
    .await?;

    if needs_helper == Some(true) {
        return Err(ScriptFailure::GameCrash.into());
    }

    scan_address(world, meta_lib, "", address).await
}

/// Lists weaknesses at `address` supported by metaxploit `version`.
pub(super) async fn scan_address(
    world: &World,
    meta_lib: &MetaLibRef,
    version: &str,
    address: &str,
) -> Result<Option<ZoneScan>> {
    Ok(
        crate::classes::meta_lib::library(world, meta_lib, |library| {
            library.scan_zone(version, address)
        })
        .await?
        .map(|scan| ZoneScan {
            zone_exists: scan.zone_exists,
            weaknesses: scan.weaknesses.iter().map(Into::into).collect(),
        }),
    )
}
