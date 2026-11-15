//! The player's monthly bank payments (`BankSubs`): what a rental or admin charge does to the rented network and its
//! administrator when it is paid and when it is not, and the charges hiring and removing an administrator through the
//! bank make. The game's payments run through its own `Payment.RunPayment` and bank page RPCs; the world's copy of the
//! game's state makes the same changes, and both are compared by money, network, administrator and payments.

use greybel_test_support::future::block_on;
use greybel_test_support::game::Game;

use crate::support::game::{setup_player, test_client};
use crate::support::intrinsics::snapshot_world;
use crate::support::parity_bank_payments::{
    Observed, observed_game, observed_world, pay, rent_network, set_money,
};

/// A rental charge the account pays enables the network it lists again, and one it cannot pay disables it, whatever
/// the account's other charges (`BankSubs.Payment.ProcessPayment`, `BankSubs.Payment.DisableNetwork`).
#[tokio::test]
async fn should_match_the_game_when_a_rental_charge_is_paid_and_when_it_is_not() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let address = rent_network(&client, std::process::id() % 10000).await;

    client.disable_network(&address, true).await.unwrap();

    let before = client.network_state().await.unwrap();
    let number = before.player_bank.as_ref().unwrap().clone();
    let mut world = snapshot_world(&user, &before);

    assert_eq!(
        observed_world(&world, &number, &address),
        observed_game(&before)
    );

    // Paid: the money is charged, the network enabled and the next payment a month later.
    pay(&client, &mut world).await;

    let paid = observed_game(&client.network_state().await.unwrap());

    assert!(!paid.disabled);
    assert_eq!(observed_world(&world, &number, &address), paid);

    // Not paid: the network is disabled and the money stays.
    set_money(&client, &mut world, &number, 10.0).await;
    pay(&client, &mut world).await;

    let unpaid = observed_game(&client.network_state().await.unwrap());

    assert!(unpaid.disabled);
    assert_eq!(unpaid.money, 10.0);
    assert_eq!(observed_world(&world, &number, &address), unpaid);
    client.shutdown();
}

/// An admin charge the account cannot pay removes nothing: the administrator stays and the network is disabled only
/// by its rental charge (`BankSubs.Payment.ProcessPayment`, which removes the administrators of the networks an admin
/// charge lists, and none is listed).
#[tokio::test]
async fn should_match_the_game_when_an_admin_charge_cannot_be_paid() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let address = rent_network(&client, std::process::id() % 10000).await;

    client.serve_rental_admin(&address, true).await.unwrap();

    let before = client.network_state().await.unwrap();
    let number = before.player_bank.as_ref().unwrap().clone();
    let mut world = snapshot_world(&user, &before);
    let hired = observed_game(&before);

    assert!(hired.admin);
    assert_eq!(hired.payments.len(), 2);
    assert_eq!(observed_world(&world, &number, &address), hired);

    set_money(&client, &mut world, &number, 0.0).await;
    pay(&client, &mut world).await;

    let unpaid = observed_game(&client.network_state().await.unwrap());

    assert!(unpaid.admin);
    assert!(unpaid.disabled);
    assert_eq!(observed_world(&world, &number, &address), unpaid);
    client.shutdown();
}

/// Hiring an administrator through the bank pays the first month and registers the charge, removing him takes the
/// charge off, and what the bank refuses changes nothing (`BankHelperServer.HireAdminRentalServerRpc`,
/// `BankHelperServer.FireAdminRentalServerRpc`).
#[tokio::test]
async fn should_match_the_game_when_an_admin_is_hired_and_removed_through_the_bank() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let address = rent_network(&client, std::process::id() % 10000).await;
    let before = client.network_state().await.unwrap();
    let number = before.player_bank.as_ref().unwrap().clone();
    let mut world = snapshot_world(&user, &before);

    // Removing an administrator that is not hired.
    client.serve_rental_admin(&address, false).await.unwrap();
    assert!(
        block_on(world.modules.npcs.jobs.fire_rental_admin(&address))
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_eq!(
        observed_world(&world, &number, &address),
        observed_game(&client.network_state().await.unwrap())
    );

    // Hiring one, then hiring another over him.
    client.serve_rental_admin(&address, true).await.unwrap();
    assert!(
        block_on(world.modules.npcs.jobs.hire_rental_admin(&address))
            .unwrap()
            .unwrap()
            .is_ok()
    );

    let hired = observed_game(&client.network_state().await.unwrap());

    assert!(hired.admin);
    assert_eq!(observed_world(&world, &number, &address), hired);

    client.serve_rental_admin(&address, true).await.unwrap();
    assert!(
        block_on(world.modules.npcs.jobs.hire_rental_admin(&address))
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_eq!(
        observed_world(&world, &number, &address),
        observed_game(&client.network_state().await.unwrap())
    );

    // Removing him.
    client.serve_rental_admin(&address, false).await.unwrap();
    assert!(
        block_on(world.modules.npcs.jobs.fire_rental_admin(&address))
            .unwrap()
            .unwrap()
            .is_ok()
    );

    let removed = observed_game(&client.network_state().await.unwrap());

    assert!(!removed.admin);
    assert_eq!(observed_world(&world, &number, &address), removed);

    // Without the money for the first month.
    set_money(&client, &mut world, &number, 100.0).await;
    client.serve_rental_admin(&address, true).await.unwrap();
    assert!(
        block_on(world.modules.npcs.jobs.hire_rental_admin(&address))
            .unwrap()
            .unwrap()
            .is_err()
    );

    let refused = observed_game(&client.network_state().await.unwrap());

    assert!(!refused.admin);
    assert_eq!(refused.money, 100.0);
    assert_eq!(observed_world(&world, &number, &address), refused);
    client.shutdown();
}

/// An administrator hired without a registered charge cannot be removed through the bank, which has no charge to
/// take off (`BankHelperServer.FireAdminRentalServerRpc`, `BankSubs.RemoveAdminPayment`).
#[tokio::test]
async fn should_match_the_game_when_the_admin_has_no_registered_charge() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let address = rent_network(&client, std::process::id() % 10000).await;

    client.hire_rental_admin(&address).await.unwrap();

    let before = client.network_state().await.unwrap();
    let number = before.player_bank.as_ref().unwrap().clone();
    let world = snapshot_world(&user, &before);
    let hired: Observed = observed_game(&before);

    assert!(hired.admin);
    assert_eq!(hired.payments.len(), 1);
    assert_eq!(observed_world(&world, &number, &address), hired);

    client.serve_rental_admin(&address, false).await.unwrap();
    assert!(
        block_on(world.modules.npcs.jobs.fire_rental_admin(&address))
            .unwrap()
            .unwrap()
            .is_err()
    );

    let after = observed_game(&client.network_state().await.unwrap());

    assert!(after.admin);
    assert_eq!(observed_world(&world, &number, &address), after);
    client.shutdown();
}
