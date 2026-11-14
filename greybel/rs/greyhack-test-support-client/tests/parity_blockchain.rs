//! The blockchain: wallets, coins, subwallets, trades and mining.

use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;

use crate::support::game::{setup_player, test_client};
use crate::support::parity::compare_in;

/// Wallet, coin, sub-wallet, market and mining success flows on the game's own economy fixture.
#[tokio::test]
async fn should_match_intrinsics_for_blockchain_wallets_and_coins() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "blockchain.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let suffix = std::process::id() % 10000;
    let coin = format!("Gb{suffix}");
    let wallet = format!("n{suffix}");
    let other = format!("x{suffix}");

    client.setup_blockchain(&coin, &wallet).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let address = snapshot.public_ip.to_string();
    // The game resumes a mining script before it saves the coin's mined total (`BlockchainSystem.CheckBlocks` calls
    // `FinishMining` before `SyncCoin`), so the script waits before it reads the total.
    let script = format!(
        "chain = include_lib(\"/lib/blockchain.so\"); w = chain.create_wallet(\"{wallet}\", \"matrix1\"); print typeof(w); print chain.create_wallet(\"{wallet}\", \"matrix1\"); print chain.create_wallet(\"{other}\", \"matrix1\"); print typeof(chain.login_wallet(\"{wallet}\", \"matrix1\")); pin = w.get_pin; print [pin.len, w.get_balance, w.list_coins, w.list_global_coins.len > 0]; c = chain.get_coin(\"{coin}\", \"{user}\", \"matrix1\"); print typeof(c); print chain.get_coin_name(\"{user}\", \"matrix1\"); print c.get_address; print c.set_address(\"{address}\"); print c.get_address; print c.set_reward(2); print c.get_reward; print c.create_subwallet(\"{wallet}\", pin, \"alpha\", \"pass1\"); print c.create_subwallet(\"{wallet}\", pin, \"beta\", \"pass2\"); a = c.get_subwallet(\"alpha\"); b = c.get_subwallet(\"beta\"); print [a.get_balance, b.get_balance, w.get_balance(\"{coin}\")]; print w.buy_coin(\"{coin}\", 5, 5, \"alpha\"); print [a.get_balance, w.get_balance(\"{coin}\")]; print w.sell_coin(\"{coin}\", 5, 5, \"alpha\"); print [a.get_balance, w.get_balance(\"{coin}\")]; print c.transaction(\"seller\", \"alpha\", 10); print c.transaction(\"alpha\", \"beta\", 3); print [a.get_balance, b.get_balance, w.get_balance(\"{coin}\")]; print a.last_transaction[:3]; print a.set_info(\"main\"); print a.get_info; print [a.check_password(\"pass1\"), a.check_password(\"bad\"), a.wallet_username]; print w.show_nodes(\"{coin}\"); print a.mining; wait(0.1); print [a.get_balance, b.get_balance, w.get_balance(\"{coin}\"), c.get_mined_coins]; print w.show_nodes(\"{coin}\"); print c.set_cycle_mining(1); print c.get_cycle_mining; print b.delete; print w.get_balance(\"{coin}\"); print chain.delete_coin(\"{coin}\", \"{user}\", \"matrix1\"); print chain.coin_price(\"{coin}\")"
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// The remaining wallet, sub-wallet, coin and blockchain queries and credential failures on the economy fixture.
#[tokio::test]
async fn should_match_intrinsics_for_blockchain_queries_and_credentials() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "blockchain.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let suffix = std::process::id() % 10000;
    let coin = format!("Qb{suffix}");
    let foreign = format!("Qf{suffix}");
    let wallet = format!("q{suffix}");

    client.setup_blockchain(&coin, &wallet).await.unwrap();
    client.setup_wallet(&wallet, "matrix1").await.unwrap();
    client
        .setup_foreign_coin(&foreign, "foreign", "foreignpass")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let foreign_row = snapshot
        .coins
        .iter()
        .find(|coin| coin.name == foreign)
        .expect("the foreign coin is mirrored");

    assert_eq!(foreign_row.creator_user, "foreign");
    assert_eq!(foreign_row.creator_password, "foreignpass");

    let scripts = vec![
        format!(
            r#"chain = include_lib("/lib/blockchain.so")
w = chain.login_wallet("{wallet}", "matrix1")
c = chain.get_coin("{coin}", "{user}", "matrix1")
other = chain.get_coin("{foreign}", "foreign", "foreignpass")
print [typeof(w), typeof(c), typeof(other)]
print chain.amount_mined("{coin}")
print chain.amount_mined("missing")
print chain.show_history("{coin}")
print chain.show_history("missing")
print w.get_pending_trade("{coin}")
print w.get_pending_trade("missing")
print w.get_global_offers("{coin}")
print w.get_global_offers("missing")
print w.cancel_pending_trade("{coin}")
print w.cancel_pending_trade("missing")"#
        ),
        format!(
            r#"chain = include_lib("/lib/blockchain.so")
w = chain.login_wallet("{wallet}", "matrix1")
c = chain.get_coin("{coin}", "{user}", "matrix1")
print c.create_subwallet("{wallet}", w.get_pin, "alpha", "pass1")
print c.transaction("seller", "alpha", 10)
subs = c.get_subwallets
for sub in subs
  print [sub.get_user, sub.get_balance]
end for
print w.sell_coin("{coin}", 1, 1000, "alpha")
print w.get_pending_trade("{coin}")
print w.get_global_offers("{coin}")
print w.cancel_pending_trade("{coin}")
print w.get_pending_trade("{coin}")
print w.cancel_pending_trade("{coin}")
print w.buy_coin("{coin}", 1, 1, "alpha")
print w.get_pending_trade("{coin}")
print w.cancel_pending_trade("{coin}")
a = c.get_subwallet("alpha")
print a.delete
print a.delete
print a.get_user
print a.get_balance"#
        ),
        format!(
            r#"chain = include_lib("/lib/blockchain.so")
w = chain.login_wallet("{wallet}", "matrix1")
c = chain.get_coin("{coin}", "{user}", "matrix1")
other = chain.get_coin("{foreign}", "foreign", "foreignpass")
print w.reset_password("bad pass")
print w.reset_password("12345678901234567")
print w.reset_password("newpass1")
print w.reset_password("again2")
print chain.login_wallet("{wallet}", "matrix1")
print typeof(chain.login_wallet("{wallet}", "newpass1"))
counter = chain.login_wallet("c{wallet}", "counterpass")
print counter.reset_password("newpass3")"#
        ),
        format!(
            r#"chain = include_lib("/lib/blockchain.so")
c = chain.get_coin("{coin}", "{user}", "matrix1")
other = chain.get_coin("{foreign}", "foreign", "foreignpass")
print c.reset_password("bad pass")
print c.reset_password("12345678901234567")
print c.reset_password("newcoin1")
print chain.get_coin("{coin}", "{user}", "matrix1")
c = chain.get_coin("{coin}", "{user}", "newcoin1")
print typeof(c)"#
        ),
        format!(
            r#"chain = include_lib("/lib/blockchain.so")
other = chain.get_coin("{foreign}", "foreign", "foreignpass")
print other.reset_password("newpass4")"#
        ),
        format!(
            r#"chain = include_lib("/lib/blockchain.so")
c = chain.get_coin("{foreign}", "foreign", "foreignpass")
print chain.delete_coin("{foreign}", "foreign", "foreignpass")
print c.reset_password("another1")"#
        ),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}
