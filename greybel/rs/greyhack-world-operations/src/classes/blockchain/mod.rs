//! Blockchain operations (`BlockchainIntrinsics`) that require an online host computer.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{ComputerId, Connectivity, WalletId, World};

use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{Access, CoinRef, Object, ObjectTable};
use crate::protocol::{
    BlockchainAmountMined, BlockchainCoinPrice, BlockchainCreateWallet, BlockchainDeleteCoin,
    BlockchainGetCoin, BlockchainGetCoinName, BlockchainLoginWallet, BlockchainShowHistory,
    CoinLogin, EconomyFailure, Failure, GameServer, WalletLogin,
};

mod coins;
mod logins;
use self::coins::{coin_price, mined_amount, show_history};
use self::logins::{create_wallet, delete_coin, get_coin, get_coin_name, login_wallet};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: BlockchainCoinPrice<'static>, caller| {
            coin_price(
                world,
                objects.blockchain(message.blockchain)?,
                &caller.terminal_computers,
                &message.coin,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: BlockchainShowHistory<'static>, caller| {
            show_history(
                world,
                objects.blockchain(message.blockchain)?,
                &caller.terminal_computers,
                &message.coin,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: BlockchainAmountMined<'static>, caller| {
            mined_amount(
                world,
                objects.blockchain(message.blockchain)?,
                &caller.terminal_computers,
                &message.coin,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: BlockchainGetCoin<'static>, caller| {
            let access = objects.blockchain(message.blockchain)?;
            let computer = access.computer;
            let login = get_coin(
                world,
                access,
                &caller.terminal_computers,
                message.user.as_deref(),
                message.password.as_deref(),
                message.coin.as_deref(),
            )
            .await?;

            Ok(match login {
                Login::Null => CoinLogin::Null,
                Login::Object(coin) => {
                    CoinLogin::Coin(objects.add(Object::Coin(CoinRef { coin, computer })))
                }
                Login::Failure(failure) => CoinLogin::Failure(failure),
            })
        },
    );
    connection.change(
        channel,
        async |world, objects, message: BlockchainCreateWallet<'static>, caller| {
            let login = create_wallet(
                world,
                objects.blockchain(message.blockchain)?,
                objects.session(caller.session)?.computer,
                &caller.terminal_computers,
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await?;

            Ok(wallet(objects, login))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: BlockchainLoginWallet<'static>, caller| {
            let login = login_wallet(
                world,
                objects.blockchain(message.blockchain)?,
                &caller.terminal_computers,
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await?;

            Ok(wallet(objects, login))
        },
    );
    connection.change(
        channel,
        async |world, objects, message: BlockchainDeleteCoin<'static>, caller| {
            delete_coin(
                world,
                objects.blockchain(message.blockchain)?,
                &caller.terminal_computers,
                message.coin.as_deref(),
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: BlockchainGetCoinName<'static>, caller| {
            get_coin_name(
                world,
                objects.blockchain(message.blockchain)?,
                &caller.terminal_computers,
                &objects.session(caller.session)?.user,
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await
        },
    );
}

/// Converts `login` into a wallet object response.
fn wallet(objects: &mut ObjectTable, login: Login<WalletId>) -> WalletLogin {
    match login {
        Login::Null => WalletLogin::Null,
        Login::Object(wallet) => WalletLogin::Wallet(objects.add(Object::Wallet(wallet))),
        Login::Failure(failure) => WalletLogin::Failure(failure),
    }
}

/// Result of a script coin or wallet login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Login<T> {
    Null,
    Object(T),
    /// Login failure returned to the script.
    Failure(Failure),
}

/// Returns [`EconomyFailure::NoInternet`] unless the blockchain computer is online.
pub(super) async fn require_online(
    world: &World,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
) -> Result<Result<(), Failure>> {
    if world
        .connectivity(blockchain.computer, terminal_computers)
        .await?
        == Connectivity::Online
    {
        Ok(Ok(()))
    } else {
        Ok(Err(Failure::Economy(EconomyFailure::NoInternet)))
    }
}
