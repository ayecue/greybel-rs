//! Exploit shop and coin grants (`admin.so`).

use greyhack_behaviour::{LibraryKind, is_valid_version};
use greyhack_world::{ComputerId, ExploitPurchase, ExploitShopFailure, World};

use crate::Result;
use crate::protocol::{AdminBuyExploit, ExploitListing, ExploitOffer, Failure, Outcome};

/// Lists the shop's offers for library file `library` at `version`
/// (`NetworkHelperServer.GetExploitListServerRpc`); a library the game has no number for has no results.
///
/// # Errors
///
/// Returns errors from database reads and writes.
pub(super) async fn exploit_shop(
    world: &mut World,
    library: &str,
    version: &str,
) -> Result<ExploitListing> {
    let result = match LibraryKind::of_file(library) {
        Some(kind) => world.exploit_offers(kind, version).await?,
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
        Err(failure) => ExploitListing::Failure(Failure::Shop(failure.into())),
    })
}

/// Buys the exploit `message` names into `computer`'s files as `user`
/// (`BankHelperServer.BuyItemServerRpc`); a library the game has no number for sells nothing.
///
/// # Errors
///
/// Returns errors from database reads and writes.
pub(super) async fn buy_exploit(
    world: &mut World,
    computer: ComputerId,
    user: &str,
    terminal_computers: &[ComputerId],
    message: &AdminBuyExploit<'_>,
) -> Result<Outcome> {
    let Some(library) = LibraryKind::of_file(&message.library) else {
        return Ok(Outcome::Failure(Failure::Shop(
            ExploitShopFailure::UnknownExploit.into(),
        )));
    };
    let sale = world
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

    Ok(Outcome::from(
        sale.map(|_| ())
            .map_err(|failure| Failure::Shop(failure.into())),
    ))
}

/// Sets the `coin` balance for `wallet_user` and reports whether the wallet exists.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn grant_coins(
    world: &mut World,
    wallet_user: &str,
    coin: &str,
    amount: f64,
) -> Result<bool> {
    let Some(wallet) = world.wallet_by_user(wallet_user).await? else {
        return Ok(false);
    };
    let id = wallet.id;

    Ok(world.set_balance(id, coin, amount).await?)
}
