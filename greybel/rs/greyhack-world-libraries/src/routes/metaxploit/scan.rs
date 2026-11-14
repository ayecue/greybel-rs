//! Library scans (`MetaExploitIntrinsics.scan`, `scan_address`).

use greyhack_world_router::{Call, RouteContext};

use crate::objects::MetaLibRef;
use crate::routes::protocol::{IntoProtocol, LibraryScan, PrintLine, ScriptFailure, ZoneScan};
use crate::routes::{OperationError, Result};

/// Lists weak addresses supported by metaxploit `version` and flags unscanned zones that may contain
/// weaknesses.
pub(super) async fn scan<W: crate::LibraryContext>(
    ctx: &RouteContext<'_, W>,
    call: Call,
    meta_lib: &MetaLibRef,
    version: &str,
) -> Result<Option<LibraryScan>> {
    let Some(library) =
        crate::routes::meta_lib::library(&*ctx.modules, meta_lib, Clone::clone).await?
    else {
        return Ok(None);
    };

    Ok(ctx
        .modules
        .libraries()
        .metaxploit
        .scan_library(&library, version)
        .await?
        .map(|scan| {
            if scan.hidden_zones {
                ctx.print(call, PrintLine::HiddenZonesWarning);
            }

            LibraryScan {
                addresses: scan.addresses,
            }
        }))
}

/// Scans `address` without a metaxploit helper; the game dereferences the helper only when weaknesses exist.
pub(super) async fn scan_address_without_metaxploit(
    world: &impl crate::LibraryContext,
    meta_lib: &MetaLibRef,
    address: &str,
) -> Result<Option<ZoneScan>> {
    let needs_helper = crate::routes::meta_lib::library(world, meta_lib, |library| {
        library.has_weaknesses_at(address)
    })
    .await?;

    if needs_helper == Some(true) {
        return Err(OperationError::Script(ScriptFailure::GameCrash));
    }

    scan_address(world, meta_lib, "", address).await
}

/// Lists weaknesses at `address` supported by metaxploit `version`.
pub(super) async fn scan_address(
    world: &impl crate::LibraryContext,
    meta_lib: &MetaLibRef,
    version: &str,
    address: &str,
) -> Result<Option<ZoneScan>> {
    Ok(
        crate::routes::meta_lib::library(world, meta_lib, |library| {
            library.scan_zone(version, address)
        })
        .await?
        .map(|scan| ZoneScan {
            zone_exists: scan.zone_exists,
            weaknesses: scan
                .weaknesses
                .iter()
                .map(IntoProtocol::into_protocol)
                .collect(),
        }),
    )
}
