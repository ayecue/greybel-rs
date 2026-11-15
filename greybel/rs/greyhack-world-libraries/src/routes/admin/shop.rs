//! Exploit shop and coin grants (`admin.so`).

use greyhack_behaviour::{LibraryKind, is_valid_version};
use greyhack_world_core::ComputerId;

use crate::routes::Result;
use crate::routes::protocol::{
    AdminBuyExploit, ExploitListing, ExploitOffer, Failure, IntoProtocol as _, Outcome,
};
use crate::{ExploitPurchase, ExploitShopFailure};

/// Lists the shop's offers for library file `library` at `version`
/// (`NetworkHelperServer.GetExploitListServerRpc`); a library the game has no number for has no results.
///
/// # Errors
///
/// Returns errors from database reads and writes.
pub(super) async fn exploit_shop(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    library: &str,
    version: &str,
) -> Result<ExploitListing> {
    let result = match LibraryKind::of_file(library) {
        Some(kind) => {
            world
                .module::<crate::LibraryModule>()
                .shop
                .exploit_offers(kind, version)
                .await?
        }
        None if is_valid_version(version) => Err(ExploitShopFailure::NoResults),
        None => Err(ExploitShopFailure::InvalidVersion),
    };

    Ok(match result {
        Ok(offers) => ExploitListing::Offers(
            offers
                .into_iter()
                .map(|offer| ExploitOffer {
                    name: offer.name,
                    description: offer.description,
                    price: offer.price,
                })
                .collect(),
        ),
        Err(failure) => ExploitListing::Failure(Failure::Shop(failure.into_protocol())),
    })
}

/// Buys the exploit `message` names into `computer`'s files as `user`
/// (`BankHelperServer.BuyItemServerRpc`); a library the game has no number for sells nothing.
///
/// # Errors
///
/// Returns errors from database reads and writes.
pub(super) async fn buy_exploit(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    computer: ComputerId,
    user: &str,
    terminal_computers: &[ComputerId],
    message: &AdminBuyExploit<'_>,
) -> Result<Outcome> {
    let Some(library) = LibraryKind::of_file(&message.library) else {
        return Ok(Outcome::Failure(Failure::Shop(
            ExploitShopFailure::UnknownExploit.into_protocol(),
        )));
    };
    let sale = world
        .module::<crate::LibraryModule>()
        .shop
        .buy_exploit(
            computer,
            user,
            terminal_computers,
            &ExploitPurchase {
                library,
                version: &message.version,
                exploit: &message.exploit,
                folder: &message.folder,
                compile: message.compile,
            },
        )
        .await?;

    Ok(sale.map_or_else(
        |failure| Outcome::Failure(Failure::Shop(failure.into_protocol())),
        |_| Outcome::True,
    ))
}
