//! File encryption, SMTP lookup, and Wi-Fi capture operations (`CryptoIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};

use crate::connection::Connection;
use crate::protocol::{
    CryptoAircrack, CryptoAireplay, CryptoAirmon, CryptoDecipher, CryptoDecrypt, CryptoEncrypt,
    CryptoIsEncrypted, CryptoSmtpUserList, GameServer,
};

mod files;
mod smtp;
mod wifi;
use self::files::{decrypt, encrypt, is_encrypted};
use self::smtp::smtp_user_list;
use self::wifi::{aircrack, aireplay, airmon};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, _, message: CryptoDecipher<'static>, _| {
            Ok(world.password_of_hash(&message.hash).await?)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CryptoEncrypt<'static>, _| {
            encrypt(
                world,
                objects.crypto(message.crypto)?,
                &message.path,
                &message.password,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CryptoDecrypt<'static>, _| {
            decrypt(
                world,
                objects.crypto(message.crypto)?,
                &message.path,
                &message.password,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CryptoIsEncrypted<'static>, _| {
            is_encrypted(world, objects.crypto(message.crypto)?, &message.path).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CryptoSmtpUserList<'static>, _| {
            smtp_user_list(
                world,
                objects.crypto(message.crypto)?,
                &message.address,
                message.port,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CryptoAirmon<'static>, _| {
            airmon(
                world,
                objects.crypto(message.crypto)?,
                message.start,
                &message.device,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CryptoAireplay<'static>, _| {
            aireplay(
                world,
                objects.crypto(message.crypto)?,
                (&message.bssid, &message.essid),
                message.max_acks,
                &message.folder,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CryptoAircrack<'static>, _| {
            aircrack(world, objects.crypto(message.crypto)?, &message.path).await
        },
    );
}
