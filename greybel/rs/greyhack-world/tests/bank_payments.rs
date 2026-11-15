use greyhack_behaviour::{RENTAL_ADMIN_PRICE, SubscriptionKind, add_month};
use greyhack_world::{DatabaseOpen, World};
use greyhack_world_core::Database;
use greyhack_world_economy::PurchaseFailure;
use greyhack_world_npcs::RentalAdminFailure;

use crate::support::bank_payments::{
    A_MONTH_AND_A_DAY, FIRST, POOR, SECOND, disabled, money, payment_dates, payments_of, rent,
    rented, subscription, world,
};

/// A rental charge the account pays charges the account and enables the networks it lists
/// (`BankSubs.Payment.ProcessPayment`).
#[tokio::test]
#[expect(
    clippy::cast_precision_loss,
    reason = "The expected balance uses the game's f64 money representation"
)]
async fn should_enable_the_rented_networks_when_their_charge_is_paid() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;
    let price = rent(&mut world, "test", network, FIRST).await;
    let started = world.handle.time_seconds();

    world
        .modules
        .core
        .networks
        .settings
        .disable_network(FIRST, true)
        .await
        .unwrap();
    world.modules.routines.advance_clock(A_MONTH_AND_A_DAY);
    world
        .modules
        .economy
        .bank
        .run_subscription_payments()
        .await
        .unwrap();

    let paid = world
        .modules
        .economy
        .bank
        .bank_account("test")
        .await
        .unwrap()
        .unwrap();
    let rental = subscription(&world, "test", SubscriptionKind::Rented)
        .await
        .unwrap();

    assert!(!disabled(&world, network).await);
    assert_eq!(paid.money, 3000.0 - price as f64);
    assert_eq!(paid.transactions.len(), 1);
    assert_eq!(paid.transactions[0].amount, -price);
    assert_eq!(paid.transactions[0].reason, "Monthly Payment");
    assert!(paid.transactions[0].success);
    assert_eq!(rental.next_payment_at, add_month(add_month(started)));
    assert_eq!(rental.public_ips, [FIRST]);
}

/// A rental charge the account cannot pay is recorded as failed and disables the networks it lists.
#[tokio::test]
async fn should_disable_the_rented_networks_when_their_charge_cannot_be_paid() {
    let mut world = world().await;
    let first = rented(&mut world, FIRST).await;
    let second = rented(&mut world, SECOND).await;

    rent(&mut world, POOR, first, FIRST).await;
    world
        .modules
        .economy
        .bank
        .add_subscription(POOR, SubscriptionKind::Rented, 40, SECOND)
        .await
        .unwrap();
    world.modules.routines.advance_clock(A_MONTH_AND_A_DAY);
    world
        .modules
        .economy
        .bank
        .run_subscription_payments()
        .await
        .unwrap();

    let poor = world
        .modules
        .economy
        .bank
        .bank_account(POOR)
        .await
        .unwrap()
        .unwrap();

    assert!(disabled(&world, first).await);
    assert!(disabled(&world, second).await);
    assert_eq!(poor.money, 10.0);
    assert_eq!(poor.transactions.len(), 1);
    assert!(!poor.transactions[0].success);
    assert_eq!(
        subscription(&world, POOR, SubscriptionKind::Rented)
            .await
            .unwrap()
            .public_ips,
        [FIRST, SECOND]
    );
}

/// An admin charge that is not paid changes no network and removes no administrator: the game lists no network on an
/// admin subscription (`BankSubs.AddPayment`), and removes administrators only of the networks listed.
#[tokio::test]
async fn should_keep_the_admin_when_the_admin_charge_cannot_be_paid() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;

    world
        .modules
        .npcs
        .jobs
        .hire_rental_admin(FIRST)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let admin = subscription(&world, "test", SubscriptionKind::Admin)
        .await
        .unwrap();

    assert_eq!(admin.amount, RENTAL_ADMIN_PRICE);
    assert!(admin.public_ips.is_empty());

    // The player spends the rest of their money.
    world
        .modules
        .economy
        .bank
        .deposit("test", -(money(&world, "test").await))
        .await
        .unwrap();
    world.modules.routines.advance_clock(A_MONTH_AND_A_DAY);
    world
        .modules
        .economy
        .bank
        .run_subscription_payments()
        .await
        .unwrap();

    let bank = world
        .modules
        .economy
        .bank
        .bank_account("test")
        .await
        .unwrap()
        .unwrap();

    assert!(!bank.transactions.last().unwrap().success);
    assert!(!disabled(&world, network).await);
    assert!(
        world
            .modules
            .npcs
            .jobs
            .has_super_admin(network)
            .await
            .unwrap()
    );
    assert!(
        subscription(&world, "test", SubscriptionKind::Admin)
            .await
            .is_some()
    );
}

/// A bank fee that is not paid leaves the networks of the other subscriptions alone.
#[tokio::test]
async fn should_not_change_networks_for_bank_fees() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;

    world
        .modules
        .economy
        .bank
        .add_subscription(POOR, SubscriptionKind::Bank, 50, "")
        .await
        .unwrap();
    world
        .modules
        .core
        .networks
        .settings
        .disable_network(FIRST, true)
        .await
        .unwrap();
    world.modules.routines.advance_clock(A_MONTH_AND_A_DAY);
    world
        .modules
        .economy
        .bank
        .run_subscription_payments()
        .await
        .unwrap();

    assert!(disabled(&world, network).await);

    world
        .modules
        .core
        .networks
        .settings
        .disable_network(FIRST, false)
        .await
        .unwrap();
    world.modules.routines.advance_clock(A_MONTH_AND_A_DAY);
    world
        .modules
        .economy
        .bank
        .run_subscription_payments()
        .await
        .unwrap();

    assert!(!disabled(&world, network).await);
}

/// A subscription lists an address once, and is removed when its amount cancels to 0 (`BankSubs.AddPayment`,
/// `BankSubs.Payment.AddMoneyPayment`).
#[tokio::test]
async fn should_list_a_network_once_and_drop_a_subscription_that_cancels() {
    let world = world().await;
    let kind = SubscriptionKind::Rented;

    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, 50, FIRST)
        .await
        .unwrap();
    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, 25, FIRST)
        .await
        .unwrap();
    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, 5, "")
        .await
        .unwrap();
    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, 10, SECOND)
        .await
        .unwrap();

    let rental = subscription(&world, "test", kind).await.unwrap();

    assert_eq!(rental.amount, 90);
    assert_eq!(rental.public_ips, [FIRST, SECOND]);

    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, -90, "")
        .await
        .unwrap();

    assert!(subscription(&world, "test", kind).await.is_none());
    assert!(
        !world
            .modules
            .economy
            .bank
            .add_subscription("unknown", kind, 5, "")
            .await
            .unwrap()
    );

    // A new subscription is made even for an amount that is not positive.
    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, -5, "")
        .await
        .unwrap();

    assert_eq!(subscription(&world, "test", kind).await.unwrap().amount, -5);
}

/// Removing an administrator's charge removes the admin subscription once nothing is left (`BankSubs.RemoveAdminPayment`).
#[tokio::test]
async fn should_remove_an_admins_charge_and_the_subscription_once_it_is_spent() {
    let world = world().await;
    let kind = SubscriptionKind::Admin;

    assert!(
        !world
            .modules
            .economy
            .bank
            .remove_admin_payment("test")
            .await
            .unwrap()
    );

    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, 2 * RENTAL_ADMIN_PRICE, "")
        .await
        .unwrap();

    assert!(
        world
            .modules
            .economy
            .bank
            .remove_admin_payment("test")
            .await
            .unwrap()
    );
    assert_eq!(
        subscription(&world, "test", kind).await.unwrap().amount,
        RENTAL_ADMIN_PRICE
    );
    assert!(
        world
            .modules
            .economy
            .bank
            .remove_admin_payment("test")
            .await
            .unwrap()
    );
    assert!(subscription(&world, "test", kind).await.is_none());
    assert!(
        !world
            .modules
            .economy
            .bank
            .remove_admin_payment("test")
            .await
            .unwrap()
    );

    // A charge below the admin's price goes below 0 and is removed too.
    world
        .modules
        .economy
        .bank
        .add_subscription("test", kind, 100, "")
        .await
        .unwrap();
    assert!(
        world
            .modules
            .economy
            .bank
            .remove_admin_payment("test")
            .await
            .unwrap()
    );
    assert!(subscription(&world, "test", kind).await.is_none());
}

/// Deleting a rented network takes its price and address off the rental subscription, and the admin's charge off the
/// admin subscription (`NetworkHelperServer.DeleteNetworkServerRpc`, `BankSubs.RemoveRentalPayments`).
#[tokio::test]
async fn should_take_a_deleted_rented_network_off_the_subscriptions() {
    let mut world = world().await;
    let first = rented(&mut world, FIRST).await;
    let second = rented(&mut world, SECOND).await;
    let first_price = rent(&mut world, "test", first, FIRST).await;
    let second_price = rent(&mut world, "test", second, SECOND).await;

    world
        .modules
        .npcs
        .jobs
        .hire_rental_admin(FIRST)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    // The hired administrator is part of the network's price but not of its rental charge.
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .rental_price(first)
            .await
            .unwrap(),
        first_price + RENTAL_ADMIN_PRICE
    );
    assert_eq!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .unwrap()
            .amount,
        first_price + second_price
    );

    assert!(
        world
            .modules
            .economy
            .bank
            .remove_network(FIRST)
            .await
            .unwrap()
    );

    let rental = subscription(&world, "test", SubscriptionKind::Rented)
        .await
        .unwrap();

    assert_eq!(rental.amount, second_price);
    assert_eq!(rental.public_ips, [SECOND]);
    assert!(
        subscription(&world, "test", SubscriptionKind::Admin)
            .await
            .is_none()
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(FIRST)
            .await
            .unwrap()
            .is_none()
    );

    assert!(
        world
            .modules
            .economy
            .bank
            .remove_network(SECOND)
            .await
            .unwrap()
    );
    assert!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .is_none()
    );
}

/// The game fails the deletion when the admin subscription is missing, after it changed the rental subscription.
#[tokio::test]
async fn should_keep_the_network_when_the_admin_subscription_is_missing() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;
    let price = rent(&mut world, "test", network, FIRST).await;

    world
        .modules
        .npcs
        .jobs
        .hire_rental_admin(FIRST)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        world
            .modules
            .economy
            .bank
            .remove_admin_payment("test")
            .await
            .unwrap()
    );
    assert_eq!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .unwrap()
            .amount,
        price
    );
    assert!(
        !world
            .modules
            .economy
            .bank
            .remove_network(FIRST)
            .await
            .unwrap()
    );
    assert!(
        world
            .modules
            .core
            .networks
            .routers
            .router_at(FIRST)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .is_none()
    );
}

/// Networks nobody rents are deleted without touching any subscription.
#[tokio::test]
async fn should_delete_a_network_nobody_rents_without_touching_subscriptions() {
    let mut world = world().await;
    let rented_network = rented(&mut world, FIRST).await;
    let other = rented(&mut world, SECOND).await;
    let price = rent(&mut world, "test", rented_network, FIRST).await;

    assert!(
        world
            .modules
            .economy
            .bank
            .remove_network(SECOND)
            .await
            .unwrap()
    );
    assert!(
        world
            .modules
            .core
            .networks
            .settings
            .network(other)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .unwrap()
            .amount,
        price
    );
}

/// Changing the speed of a rented network moves the rental charge by the packs' price difference
/// (`NetworkHelperServer.RefreshRentedPayments`); other networks change nothing.
#[tokio::test]
async fn should_move_the_rental_charge_with_the_speed_pack() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;
    let other = rented(&mut world, SECOND).await;
    let price = rent(&mut world, "test", network, FIRST).await;

    world
        .modules
        .economy
        .bank
        .set_net_speed(FIRST, 25)
        .await
        .unwrap();

    // Pack M costs 25 and pack XXL 170.
    assert_eq!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .unwrap()
            .amount,
        price + 145
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .rental_price(network)
            .await
            .unwrap(),
        price + 145
    );

    world
        .modules
        .economy
        .bank
        .set_net_speed(FIRST, 3)
        .await
        .unwrap();

    assert_eq!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .unwrap()
            .amount,
        price - 15
    );

    world
        .modules
        .economy
        .bank
        .set_net_speed(SECOND, 25)
        .await
        .unwrap();

    assert_eq!(
        subscription(&world, "test", SubscriptionKind::Rented)
            .await
            .unwrap()
            .amount,
        price - 15
    );
    assert_eq!(
        world
            .modules
            .core
            .networks
            .settings
            .network(other)
            .await
            .unwrap()
            .unwrap()
            .net_speed,
        25
    );
}

/// Hiring an administrator is paid from the player's bank account with no coupons and registers the monthly charge
/// (`BankHelperServer.HireAdminRentalServerRpc`); firing takes the charge off.
#[tokio::test]
#[expect(
    clippy::cast_precision_loss,
    reason = "The expected balance uses the game's f64 money representation"
)]
async fn should_charge_the_player_for_an_admin_and_register_the_monthly_charge() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;
    let coupons = world
        .modules
        .libraries
        .shop
        .coupons(world.handle.player())
        .await
        .unwrap();
    let price = world
        .modules
        .economy
        .bank
        .rental_price(network)
        .await
        .unwrap();

    world
        .modules
        .npcs
        .jobs
        .hire_rental_admin(FIRST)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let bank = world
        .modules
        .economy
        .bank
        .bank_account("test")
        .await
        .unwrap()
        .unwrap();
    let admin = subscription(&world, "test", SubscriptionKind::Admin)
        .await
        .unwrap();

    assert_eq!(bank.money, 3000.0 - RENTAL_ADMIN_PRICE as f64);
    assert_eq!(
        bank.transactions.last().unwrap().reason,
        "Hire admin services"
    );
    assert_eq!(
        bank.transactions.last().unwrap().amount,
        -RENTAL_ADMIN_PRICE
    );
    assert_eq!(
        world
            .modules
            .libraries
            .shop
            .coupons(world.handle.player())
            .await
            .unwrap(),
        coupons
    );
    assert_eq!(admin.amount, RENTAL_ADMIN_PRICE);
    assert_eq!(
        admin.next_payment_at,
        add_month(world.handle.time_seconds())
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .rental_price(network)
            .await
            .unwrap(),
        price + RENTAL_ADMIN_PRICE
    );

    world
        .modules
        .npcs
        .jobs
        .fire_rental_admin(FIRST)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert!(
        subscription(&world, "test", SubscriptionKind::Admin)
            .await
            .is_none()
    );
    assert!(
        !world
            .modules
            .npcs
            .jobs
            .has_super_admin(network)
            .await
            .unwrap()
    );
    assert_eq!(
        world
            .modules
            .npcs
            .jobs
            .fire_rental_admin(FIRST)
            .await
            .unwrap(),
        Some(Err(RentalAdminFailure::NoAdmin))
    );
}

/// Without funds for the first month no administrator is hired and nothing is registered.
#[tokio::test]
async fn should_not_hire_an_admin_the_player_cannot_pay() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;
    let other = rented(&mut world, SECOND).await;

    world
        .modules
        .npcs
        .jobs
        .hire_rental_admin(FIRST)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let refused = world
        .modules
        .npcs
        .jobs
        .hire_rental_admin(SECOND)
        .await
        .unwrap();

    assert_eq!(
        refused,
        Some(Err(RentalAdminFailure::Purchase(
            PurchaseFailure::InsufficientFunds
        )))
    );
    assert!(
        !world
            .modules
            .npcs
            .jobs
            .has_super_admin(other)
            .await
            .unwrap()
    );
    assert!(
        world
            .modules
            .npcs
            .jobs
            .has_super_admin(network)
            .await
            .unwrap()
    );
    assert_eq!(
        subscription(&world, "test", SubscriptionKind::Admin)
            .await
            .unwrap()
            .amount,
        RENTAL_ADMIN_PRICE
    );
    assert_eq!(money(&world, "test").await, 500.0);
}

/// An administrator hired without a monthly charge (as the network was made with one) cannot be fired through the
/// bank's page: the charge to take off is missing (`BankHelperServer.FireAdminRentalServerRpc`).
#[tokio::test]
async fn should_not_fire_an_admin_without_a_registered_charge() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;

    world
        .modules
        .npcs
        .jobs
        .hire_rental_admin(FIRST)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    world
        .modules
        .economy
        .bank
        .remove_admin_payment("test")
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .npcs
            .jobs
            .fire_rental_admin(FIRST)
            .await
            .unwrap(),
        Some(Err(RentalAdminFailure::NoAdminPayment))
    );
    assert!(
        world
            .modules
            .npcs
            .jobs
            .has_super_admin(network)
            .await
            .unwrap()
    );
}

/// A player without a bank account cannot hire.
#[tokio::test]
async fn should_not_hire_an_admin_without_a_bank_account() {
    let mut world = World::new(&greyhack_world::PlayerOptions {
        bank_number: None,
        ..greyhack_world::PlayerOptions::default()
    })
    .await
    .unwrap();

    rented(&mut world, FIRST).await;

    assert_eq!(
        world
            .modules
            .npcs
            .jobs
            .hire_rental_admin(FIRST)
            .await
            .unwrap(),
        Some(Err(RentalAdminFailure::Purchase(
            PurchaseFailure::NoBankAccount
        )))
    );
}

/// The listed networks and next dates survive saving and loading the world.
#[tokio::test]
async fn should_keep_the_listed_networks_when_the_world_is_saved_and_loaded() {
    let mut world = world().await;
    let first = rented(&mut world, FIRST).await;
    let second = rented(&mut world, SECOND).await;

    rent(&mut world, "test", first, FIRST).await;
    rent(&mut world, "test", second, SECOND).await;

    let database = Database::in_memory().await.unwrap();

    world.save_into(&database).await.unwrap();

    let loaded = World::load(&database).await.unwrap();

    assert_eq!(
        loaded
            .modules
            .economy
            .bank
            .bank_subscriptions()
            .await
            .unwrap(),
        world
            .modules
            .economy
            .bank
            .bank_subscriptions()
            .await
            .unwrap()
    );
    assert_eq!(
        subscription(&loaded, "test", SubscriptionKind::Rented)
            .await
            .unwrap()
            .public_ips,
        [FIRST, SECOND]
    );
}

/// Each payment runs at its own date, not all of them when a day ends: the charge that falls due is made and the
/// others wait for theirs (`BankSubs.Payment.WaitUntilPayment`, `BankSubs.Payment.RunPayment`).
#[tokio::test]
#[expect(
    clippy::cast_precision_loss,
    reason = "The expected balance uses the game's f64 money representation"
)]
async fn should_run_each_payment_at_its_own_date() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;
    let price = rent(&mut world, "test", network, FIRST).await;
    let started = world.handle.time_seconds();

    world.tick(10 * 86_400).await.unwrap();
    world
        .modules
        .economy
        .bank
        .add_subscription("test", SubscriptionKind::Admin, 100, "")
        .await
        .unwrap();

    let admin_due = add_month(world.handle.time_seconds());
    let report = world
        .tick(add_month(started) - world.handle.time_seconds())
        .await
        .unwrap();
    let rental = subscription(&world, "test", SubscriptionKind::Rented)
        .await
        .unwrap();
    let admin = subscription(&world, "test", SubscriptionKind::Admin)
        .await
        .unwrap();
    let paid = world
        .modules
        .economy
        .bank
        .bank_account("test")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(payments_of(&report.routines).len(), 1);
    assert_eq!(payments_of(&report.routines)[0], "economy.payment");
    assert_eq!(paid.money, 3000.0 - price as f64);
    assert_eq!(paid.transactions.len(), 1);
    assert_eq!(rental.next_payment_at, add_month(add_month(started)));
    assert_eq!(admin.next_payment_at, admin_due);
    assert_eq!(
        payment_dates(&world).await,
        [admin_due, rental.next_payment_at]
    );

    world
        .tick(admin_due - world.handle.time_seconds())
        .await
        .unwrap();

    let paid = world
        .modules
        .economy
        .bank
        .bank_account("test")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(paid.money, 3000.0 - price as f64 - 100.0);
    assert_eq!(paid.transactions.len(), 2);
}

/// A payment whose subscription ended no longer runs, and a subscription made again waits for its own date.
#[tokio::test]
async fn should_not_run_a_payment_that_was_removed() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;
    let price = rent(&mut world, "test", network, FIRST).await;

    assert!(
        world
            .modules
            .economy
            .bank
            .remove_rental_payments("test", FIRST, price, false)
            .await
            .unwrap()
    );
    assert!(payment_dates(&world).await.is_empty());

    world.modules.routines.advance_clock(A_MONTH_AND_A_DAY);

    let report = world.tick(0).await.unwrap();

    assert!(payments_of(&report.routines).is_empty());
    assert_eq!(money(&world, "test").await, 3000.0);
}

/// A payment that fell due while nothing ran is not charged when the world starts again: its date becomes a month
/// from then (`BankSubs.Payment.RestoreWaiting`, `BankSubs.Payment.SecondsNextPayment`).
#[tokio::test]
async fn should_move_a_missed_payment_a_month_on_when_payments_are_restored() {
    let mut world = world().await;
    let network = rented(&mut world, FIRST).await;

    rent(&mut world, "test", network, FIRST).await;
    world.modules.routines.advance_clock(2 * A_MONTH_AND_A_DAY);
    world.modules.economy.bank.restore_payments().await.unwrap();

    let rental = subscription(&world, "test", SubscriptionKind::Rented)
        .await
        .unwrap();
    let report = world.tick(0).await.unwrap();

    assert_eq!(
        rental.next_payment_at,
        add_month(world.handle.time_seconds())
    );
    assert!(payments_of(&report.routines).is_empty());
    assert_eq!(money(&world, "test").await, 3000.0);
    assert_eq!(payment_dates(&world).await, [rental.next_payment_at]);
}
