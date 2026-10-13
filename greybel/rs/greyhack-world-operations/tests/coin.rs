//! Tests that coin operations answer what `CoinIntrinsics.CheckStatus` and `set_address` answer before they start.

use duplex_core::Error;
use duplex_test_support::local::run_local;

use crate::support::program::{
    CoinGetReward, CoinSetAddress, EconomyFailure, Failure, ScriptFailure,
};
use crate::support::server::ether;
use crate::support::{connect, server_with_ether};

#[test]
fn should_refuse_a_coin_operation_once_its_computer_is_offline() {
    run_local(async {
        let server = server_with_ether();
        let runtime = connect(&server);
        let session = runtime.start("/bin/coin").await;
        let coin = ether(&runtime, &session).await;

        server
            .change_world(async |world| {
                let player = world.player();

                for (index, mut card) in world
                    .net_cards(player)
                    .await
                    .unwrap()
                    .into_iter()
                    .enumerate()
                {
                    card.active = false;
                    world.save_net_card(player, index, &card).await.unwrap();
                }
            })
            .await;

        let reward = runtime
            .channel
            .send(&CoinGetReward {
                program: session.program,
                coin,
            })
            .expect("the read is sent")
            .await;

        assert_eq!(
            reward,
            Err(Error::Failure(ScriptFailure::Refused(Failure::Economy(
                EconomyFailure::NoInternet
            ))))
        );
    });
}

#[test]
fn should_set_a_coin_address_only_where_a_network_is() {
    run_local(async {
        let server = server_with_ether();
        let runtime = connect(&server);
        let session = runtime.start("/bin/coin").await;
        let coin = ether(&runtime, &session).await;
        let mut answers = Vec::new();

        for address in ["1.2.3.4", "100.64.0.1", "nowhere.test"] {
            let answer = runtime
                .channel
                .send(&CoinSetAddress {
                    program: session.program,
                    coin,
                    address: address.into(),
                })
                .expect("the change is sent")
                .await;

            answers.push(answer);
        }

        assert_eq!(
            answers,
            [
                Ok(Some(true)),
                Err(Error::Failure(ScriptFailure::Refused(Failure::Economy(
                    EconomyFailure::UnknownAddress("100.64.0.1".to_owned())
                )))),
                Err(Error::Failure(ScriptFailure::Refused(Failure::Economy(
                    EconomyFailure::UnknownAddress("nowhere.test".to_owned())
                )))),
            ]
        );
    });
}
