//! When each monthly payment runs: at its own date, not all together once a day. A rental charge made first falls due
//! first, and the admin charge made ten days later waits for its own date (`BankSubs.Payment.WaitUntilPayment`,
//! `BankSubs.Payment.RunPayment`). The game runs the one payment that is due through its own `RunPayment`; the world's
//! copy runs what its routines have due when its clock gets there, and both are compared by money, network and payments.

use greybel_test_support::future::block_on;
use greybel_test_support::game::Game;

use crate::support::game::{setup_player, test_client};
use crate::support::intrinsics::snapshot_world;
use crate::support::parity_bank_payments::{observed_game, observed_world, rent_network};
use crate::support::parity_payment_timing::TEN_DAYS;

#[tokio::test]
async fn should_match_the_game_when_the_payments_fall_due_one_after_the_other() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let address = rent_network(&client, std::process::id() % 10000).await;

    client.set_player_money(10_000.0).await.unwrap();
    client.advance_clock(TEN_DAYS).await.unwrap();
    client.serve_rental_admin(&address, true).await.unwrap();

    let before = client.network_state().await.unwrap();
    let number = before.player_bank.as_ref().unwrap().to_string();
    let world = snapshot_world(&user, &before);
    let hired = observed_game(&before);

    assert_eq!(hired.payments.len(), 2);
    assert_eq!(observed_world(&world, &number, &address), hired);

    // The rental charge falls due first: the game runs that payment alone, the world's clock reaches its date.
    let rental_date = hired
        .payments
        .iter()
        .find(|payment| payment.0 == "Rented")
        .map(|payment| payment.2)
        .expect("the rental charge");
    let admin_date = hired
        .payments
        .iter()
        .find(|payment| payment.0 == "Admin")
        .map(|payment| payment.2)
        .expect("the admin charge");

    assert!(rental_date < admin_date);

    let wait = rental_date - world.handle.time_seconds();

    client.advance_clock(wait).await.unwrap();
    client.run_payment("RENTED").await.unwrap();
    block_on(world.tick(wait)).expect("the clock moves");

    let first = observed_game(&client.network_state().await.unwrap());

    assert_eq!(observed_world(&world, &number, &address), first);

    // Then the admin charge's date comes.
    let wait = admin_date - world.handle.time_seconds();

    client.advance_clock(wait).await.unwrap();
    client.run_payment("ADMIN").await.unwrap();
    block_on(world.tick(wait)).expect("the clock moves");

    let second = observed_game(&client.network_state().await.unwrap());

    assert_eq!(observed_world(&world, &number, &address), second);
    client.shutdown();
}
