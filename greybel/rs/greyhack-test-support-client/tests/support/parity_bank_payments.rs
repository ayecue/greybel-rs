//! What the bank payment comparisons share: a rented network the player owns in the game, its copy, and the state both
//! worlds are compared by.

use greybel_test_support::future::block_on;
use greyhack_behaviour::{SubscriptionKind, parse_date};
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::NetworkSnapshot;
use greyhack_world::World;

use super::game::join_network;

/// More than one month, so a payment made at the start is due.
pub const A_MONTH_AND_A_DAY: u64 = 32 * 86_400;

/// What a rental charge registered by the fixture costs a month.
pub const RENTAL_AMOUNT: i64 = 150;

/// What both worlds are compared by after something happens to a rented network and the player's payments.
#[derive(Debug, PartialEq)]
pub struct Observed {
    /// The money of the player's bank account.
    pub money: f64,
    /// Whether the rented network is disabled for nonpayment.
    pub disabled: bool,
    /// Whether the rented network's hired administrator is alive and not fired.
    pub admin: bool,
    /// Each payment of the player's account: its kind name, amount, next date in seconds and listed networks.
    pub payments: Vec<(&'static str, i64, u64, Vec<String>)>,
}

/// The player's bank account and its money in `snapshot`.
fn player_account(snapshot: &NetworkSnapshot) -> (String, f64) {
    let number = snapshot
        .player_bank
        .as_ref()
        .expect("the player's bank account")
        .to_string();
    let money = snapshot
        .banks
        .iter()
        .find(|bank| bank.number == number)
        .map(|bank| bank.money)
        .expect("the player's bank account is read");

    (number, money)
}

/// What the game has of the rented network the player is on.
pub fn observed_game(snapshot: &NetworkSnapshot) -> Observed {
    let (number, money) = player_account(snapshot);

    Observed {
        money,
        disabled: snapshot.disabled,
        admin: snapshot.rental_admin,
        payments: snapshot
            .subscriptions
            .iter()
            .filter(|payment| payment.bank == number)
            .map(|payment| {
                (
                    match payment.kind.as_ref() {
                        "BANK" => "Bank",
                        "RENTED" => "Rented",
                        "ADMIN" => "Admin",
                        other => panic!("the game's payment kind {other}"),
                    },
                    payment.amount,
                    parse_date(&payment.next_date),
                    payment.public_ips.iter().map(ToString::to_string).collect(),
                )
            })
            .collect(),
    }
}

/// What the world has of the rented network at `public_ip` and the account `number`.
pub fn observed_world(world: &World, number: &str, public_ip: &str) -> Observed {
    let router = block_on(world.router_at(public_ip))
        .expect("the router is read")
        .expect("the rented network");
    let network = block_on(world.network(router.network))
        .expect("the network is read")
        .expect("the rented network");
    let payments = block_on(world.bank_subscriptions()).expect("the payments are read");

    Observed {
        money: block_on(world.bank_account(number))
            .expect("the account is read")
            .expect("the player's account")
            .money,
        disabled: network.disabled,
        admin: block_on(world.has_super_admin(router.network)).expect("the admin is read"),
        payments: payments
            .into_iter()
            .filter(|payment| payment.bank == number)
            .map(|payment| {
                (
                    match payment.kind {
                        SubscriptionKind::Bank => "Bank",
                        SubscriptionKind::Rented => "Rented",
                        SubscriptionKind::Admin => "Admin",
                    },
                    payment.amount,
                    payment.next_payment_at,
                    payment.public_ips,
                )
            })
            .collect(),
    }
}

/// Gives the player a bank account and a rented network in the game: a `RentServer` network the player has joined,
/// charged [`RENTAL_AMOUNT`] a month, with the player's bank account at `suffix`'s coin. Returns the network's
/// address.
pub async fn rent_network(client: &TestClient, suffix: u32) -> String {
    client
        .setup_blockchain(&format!("Bp{suffix}"), &format!("b{suffix}"))
        .await
        .expect("the player's bank account");

    let network = client
        .spawn_network("RentServer")
        .await
        .expect("a rented server");
    let address = network.public_ip.to_string();

    join_network(client, &address).await;
    client
        .rent_network(&address, RENTAL_AMOUNT)
        .await
        .expect("the player rents the network");

    address
}

/// Moves both worlds a month and a day on and runs the player's payments in both.
pub async fn pay(client: &TestClient, world: &mut World) {
    client
        .advance_clock(A_MONTH_AND_A_DAY)
        .await
        .expect("the game clock moves");
    client
        .run_payments()
        .await
        .expect("the game's payments run");
    world.advance_clock(A_MONTH_AND_A_DAY);
    block_on(world.run_subscription_payments()).expect("the world's payments run");
}

/// Sets the money of the player's account to `money` in both worlds.
pub async fn set_money(client: &TestClient, world: &mut World, number: &str, money: f64) {
    client
        .set_player_money(money)
        .await
        .expect("the game's money is set");

    let now = block_on(world.bank_account(number))
        .expect("the account is read")
        .expect("the player's account")
        .money;

    block_on(world.deposit(number, money - now)).expect("the world's money is set");
}
