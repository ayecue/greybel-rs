use greyhack_behaviour::{
    NetworkEffect, RENTAL_ADMIN_PRICE, RentalDeviceTable, SubscriptionKind, net_speed_price,
    rental_price,
};

/// A paid rental or admin charge enables the networks it lists, an unpaid rental charge disables them, and the rest
/// leaves them as they are (`BankSubs.Payment.ProcessPayment`).
#[test]
fn should_decide_what_a_payment_does_to_its_networks() {
    use NetworkEffect::{Disable, Enable, Skip};

    assert_eq!(SubscriptionKind::Bank.network_effect(true), Skip);
    assert_eq!(SubscriptionKind::Bank.network_effect(false), Skip);
    assert_eq!(SubscriptionKind::Rented.network_effect(true), Enable);
    assert_eq!(SubscriptionKind::Rented.network_effect(false), Disable);
    assert_eq!(SubscriptionKind::Admin.network_effect(true), Enable);
    assert_eq!(SubscriptionKind::Admin.network_effect(false), Skip);
}

/// Speed packs cost what `ISPPanel.PackPrice` says, and another speed costs its own number.
#[test]
fn should_price_the_speed_packs() {
    assert_eq!(
        [3, 6, 12, 20, 25].map(net_speed_price),
        [10, 25, 40, 60, 170]
    );
    assert_eq!(net_speed_price(7), 7);
}

/// A rented network costs its devices, a hired administrator and its pack (`ConfigLanUI.CalculaPrecio`).
#[test]
fn should_price_a_rented_network() {
    let devices = RentalDeviceTable {
        computers: 2,
        routers: 3,
        hubs: 1,
        switches: 2,
    };

    assert_eq!(
        rental_price(devices, false, 6),
        2 * 75 + 3 * 50 + 35 + 2 * 40 + 25
    );
    assert_eq!(
        rental_price(devices, true, 6),
        rental_price(devices, false, 6) + RENTAL_ADMIN_PRICE
    );
    assert_eq!(rental_price(RentalDeviceTable::default(), false, 25), 170);
}
