//! File encryption and Wi-Fi capture operations (`CryptoIntrinsics`).

use greyhack_world_router::RouteTable;

use crate::IntrusionContext;
use crate::objects::ObjectTableExt as _;
use crate::routes::protocol::{
    CryptoAircrack, CryptoAireplay, CryptoAirmon, CryptoDecipher, CryptoDecrypt, CryptoEncrypt,
    CryptoIsEncrypted,
};

mod files;
mod wifi;
use self::files::{decrypt, encrypt, is_encrypted};
use self::wifi::{aircrack, aireplay, airmon};

pub(crate) fn register<W: IntrusionContext + 'static>(routes: &RouteTable) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: CryptoDecipher<'static>| {
            let world = &*ctx.modules;

            Ok(world
                .core()
                .computers
                .users
                .password_of_hash(&message.hash)
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CryptoEncrypt<'static>| {
            let world = &*ctx.modules;

            encrypt(
                world,
                &ctx.trees.objects(call.tree).crypto(message.crypto)?,
                &message.path,
                &message.password,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CryptoDecrypt<'static>| {
            let world = &*ctx.modules;

            decrypt(
                world,
                &ctx.trees.objects(call.tree).crypto(message.crypto)?,
                &message.path,
                &message.password,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CryptoIsEncrypted<'static>| {
            let world = &*ctx.modules;

            is_encrypted(
                world,
                &ctx.trees.objects(call.tree).crypto(message.crypto)?,
                &message.path,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CryptoAirmon<'static>| {
            let world = &*ctx.modules;

            airmon(
                world,
                &ctx.trees.objects(call.tree).crypto(message.crypto)?,
                message.start,
                &message.device,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CryptoAireplay<'static>| {
            aireplay(
                ctx,
                call,
                &ctx.trees.objects(call.tree).crypto(message.crypto)?,
                (&message.bssid, &message.essid),
                message.max_acks,
                &message.folder,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CryptoAircrack<'static>| {
            aircrack(
                ctx,
                call,
                &ctx.trees.objects(call.tree).crypto(message.crypto)?,
                &message.path,
            )
            .await
        },
    );
}
