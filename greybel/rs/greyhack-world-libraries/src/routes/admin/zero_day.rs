use greyhack_behaviour::CompanyAlign;
use greyhack_world_core::ComputerId;
use greyhack_world_programs::protocol::{
    AdminAnswerZeroDay, AdminBuyDebugOffer, AdminDebugOffer, AdminUnlockDebugLibrary,
    DebugOfferInfo, ZeroDayOffer,
};
use greyhack_world_router::{RouteContext, RouteTable};

use crate::{DebugOffer, DebugPurchase};

#[expect(
    clippy::too_many_lines,
    reason = "Each interface-only zero-day operation has its own typed route"
)]
pub(super) fn register<W: greyhack_world_router::WorldContext<Computer = ComputerId> + 'static>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminUnlockDebugLibrary<'static>| {
            let world = &*ctx.modules;
            let Some(computer) = world
                .module::<greyhack_world_core::CoreModule>()
                .networks
                .routers
                .computer_at(&message.public_ip, &message.lan_ip)
                .await?
            else {
                return Ok(false);
            };

            Ok(world
                .module::<crate::LibraryModule>()
                .debug
                .unlock_debug_library(computer, &message.path)
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminDebugOffer<'static>| {
            let debug = &ctx.modules.module::<crate::LibraryModule>().debug;
            if message.user.is_empty() {
                return Ok(debug
                    .get_best_offer_creds()
                    .await?
                    .map_or(ZeroDayOffer::Null, |offer| {
                        ZeroDayOffer::Offer(offer_info(offer))
                    }));
            }
            if message.password.is_empty()
                || message.user.encode_utf16().count() > 32
                || message.password.encode_utf16().count() > 32
                || message.price < 0
            {
                return Ok(ZeroDayOffer::Failure(
                    "Error: The username and password cannot be empty or exceed 32 characters. The price cannot be negative or empty."
                        .to_owned(),
                ));
            }
            let Some(bank) = ctx
                .modules
                .module::<greyhack_world_economy::EconomyModule>()
                .bank
                .player_bank()
                .await?
            else {
                return Ok(ZeroDayOffer::Failure(
                    "Error: It is necessary to register a bank account to sell the credentials."
                        .to_owned(),
                ));
            };
            let player = debug.player_id();
            let error = debug
                .add_offer_creds(&player, &bank, message.price, &message.user, &message.password)
                .await?;
            if !error.is_empty() {
                return Ok(ZeroDayOffer::Failure(error));
            }
            Ok(debug
                .get_best_offer_creds()
                .await?
                .map_or(ZeroDayOffer::Null, |offer| {
                    ZeroDayOffer::Offer(offer_info(offer))
                }))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminBuyDebugOffer<'static>| {
            let debug = &ctx.modules.module::<crate::LibraryModule>().debug;
            let player = debug.player_id();
            Ok(
                match debug
                    .buy_offer_creds(&message.seller, message.price, &player)
                    .await?
                {
                    DebugPurchase::Purchased(offer) => ZeroDayOffer::Offer(offer_info(offer)),
                    DebugPurchase::Refused(error) => ZeroDayOffer::Failure(error),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &RouteContext<'_, W>, _call, message: AdminAnswerZeroDay<'static>| {
            let debug = &ctx.modules.module::<crate::LibraryModule>().debug;
            let answer = match &*message.answer {
                "GOOD" => CompanyAlign::Good,
                "BAD" => CompanyAlign::Bad,
                _ => CompanyAlign::None,
            };

            Ok(debug
                .answer_zero_day(
                    &message.public_ip,
                    &message.user,
                    &message.from_mail,
                    answer,
                    &debug.player_id(),
                )
                .await?)
        },
    );
}

fn offer_info(offer: DebugOffer) -> DebugOfferInfo {
    DebugOfferInfo {
        seller: offer.seller,
        user: offer.user,
        password: offer.password,
        price: offer.price,
        expires_at: 0,
        purchased: offer.purchased,
    }
}
