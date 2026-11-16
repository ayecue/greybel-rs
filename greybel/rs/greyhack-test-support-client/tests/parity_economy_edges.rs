use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;

use crate::support::game::{setup_player, test_client};
use crate::support::parity::compare_labeled;

/// Wallet, coin, and sub-wallet boundary cases with deterministic two-coin fixtures.
#[tokio::test]
async fn should_match_intrinsics_for_economy_core_edges() {
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
    let coin = format!("C{suffix}");
    let wallet = format!("w{suffix}");
    let foreign = format!("F{suffix}");

    client.setup_blockchain(&coin, &wallet).await.unwrap();
    client.setup_wallet(&wallet, "pass").await.unwrap();
    client
        .setup_foreign_coin(&foreign, "foreign", "pass")
        .await
        .unwrap();

    let setup = "bc = include_lib(\"/lib/blockchain.so\"); w = bc.login_wallet(\"corew\", \"pass\"); c = bc.get_coin(\"Core\", \"test\", \"matrix1\"); other = bc.get_coin(\"Beta\", \"foreign\", \"pass\")"
        .replace("corew", &wallet)
        .replace("Core", &coin)
        .replace("Beta", &foreign);
    let scripts = [
        format!(
            "{setup}; print [bc.get_coin(\"x\"), bc.login_wallet(\"missing\", \"pass\"), bc.login_wallet(\"corew\", \"wrong\")]; print w.buy_coin(\"Core\", 0, 1, \"alpha\"); print c.create_subwallet(\"corew\", w.get_pin, \"alpha\", \"pass1\"); print c.transaction(\"seller\", \"alpha\", 10); print c.transaction(\"alpha\", \"beta\", 99); print other.create_subwallet(\"corew\", w.get_pin, \"beta\", \"pass1\")"
        ),
        format!(
            "{setup}; print [c.get_mined_coins, c.get_cycle_mining, bc.coin_price(\"Core\"), bc.get_coin_name(\"test\", \"matrix1\"), c.get_subwallets]; print c.reset_password(\"bad\"); print c.get_subwallet(\"alpha\").delete; print c.get_subwallet(\"alpha\")"
        ),
    ]
    .map(|script| {
        script
            .replace("corew", &wallet)
            .replace("Core", &coin)
            .replace("Beta", &foreign)
    });

    let snapshot = client.network_state().await.unwrap();

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "economy-core-validation",
        &scripts,
    )
    .await;

    let unbanked = format!("u{wallet}");

    for (fixture, script) in [
        (
            "delete_wallet",
            "bc = include_lib(\"/lib/blockchain.so\"); print bc.login_wallet(\"ccorew\", \"counterpass\")"
                .to_owned(),
        ),
        (
            "delete_subwallet",
            format!(
                "{setup}; print c.get_subwallet(\"alpha\").delete; print c.get_subwallet(\"alpha\")"
            ),
        ),
        (
            "unbanked_wallet",
            "bc = include_lib(\"/lib/blockchain.so\"); u = bc.login_wallet(\"unbanked\", \"matrix1\"); print [u.get_balance(\"Core\"), u.get_pin]"
                .replace("Core", &coin),
        ),
        ("fully_mined", format!("{setup}; print c.start_mining(0)")),
        (
            "delete_coin",
            "bc = include_lib(\"/lib/blockchain.so\"); print bc.get_coin(\"Core\")".to_owned(),
        ),
    ] {
        let script = script
            .replace("corew", &wallet)
            .replace("Core", &coin)
            .replace("Beta", &foreign)
            .replace("unbanked", &unbanked);

        let fixture_wallet = match fixture {
            "delete_wallet" => format!("c{wallet}"),
            "unbanked_wallet" => unbanked.clone(),
            _ => wallet.clone(),
        };

        let fixture_subwallet = if fixture == "unbanked_wallet" {
            unbanked.as_str()
        } else {
            "alpha"
        };

        client
            .configure_economy(fixture, &coin, &fixture_wallet, fixture_subwallet)
            .await
            .unwrap();

        let snapshot = client.network_state().await.unwrap();

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("economy-{fixture}"),
            &[script],
        )
        .await;
    }

    client.shutdown();
}

/// Partial market trades, custom orders, cancellation, price changes, and mining hardware failures.
#[tokio::test]
async fn should_match_intrinsics_for_economy_market_and_mining_edges() {
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

    let suffix = std::process::id() % 10000 + 1;
    let coin = format!("M{suffix}");
    let wallet = format!("m{suffix}");

    client.setup_blockchain(&coin, &wallet).await.unwrap();
    client.setup_wallet(&wallet, "pass").await.unwrap();

    let scripts = [
        "bc = include_lib(\"/lib/blockchain.so\"); w = bc.login_wallet(\"corew\", \"pass\"); c = bc.get_coin(\"Core\", \"test\", \"matrix1\"); c.create_subwallet(\"corew\", w.get_pin, \"alpha\", \"pass1\"); c.transaction(\"seller\", \"alpha\", 10); print w.sell_coin(\"Core\", 2, 5, \"alpha\"); print w.buy_coin(\"Core\", 2, 5, \"alpha\"); print w.get_pending_trade(\"Core\")".to_owned(),
        "bc = include_lib(\"/lib/blockchain.so\"); w = bc.login_wallet(\"corew\", \"pass\"); print w.buy_coin(\"Core\", 1, 7, \"alpha\"); print w.get_pending_trade(\"Core\"); print w.cancel_pending_trade(\"Core\"); print w.get_global_offers(\"Core\")".to_owned(),
        "bc = include_lib(\"/lib/blockchain.so\"); w = bc.login_wallet(\"corew\", \"pass\"); print w.sell_coin(\"Core\", 1, 99, \"alpha\"); print w.buy_coin(\"Core\", 0, 1, \"alpha\")".to_owned(),
    ]
    .map(|script| script.replace("corew", &wallet).replace("Core", &coin));

    let snapshot = client.network_state().await.unwrap();

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "economy-market-partial-orders",
        &scripts,
    )
    .await;

    client
        .configure_economy("damaged_gpu", &coin, &wallet, "alpha")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "economy-damaged-gpu",
        &[
            "bc = include_lib(\"/lib/blockchain.so\"); w = bc.login_wallet(\"corew\", \"pass\"); c = bc.get_coin(\"Core\", \"test\", \"matrix1\"); print c.start_mining(0)".to_owned(),
            "bc = include_lib(\"/lib/blockchain.so\"); w = bc.login_wallet(\"corew\", \"pass\"); c = bc.get_coin(\"Core\", \"test\", \"matrix1\"); print c.stop_mining".to_owned(),
        ],
    )
    .await;

    client.shutdown();
}
