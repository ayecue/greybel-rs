use greyhack_behaviour::{NetworkKind, RENTAL_ADMIN_PRICE, SubscriptionKind};

use crate::RentalAdminFailure;
use crate::systems::npcs::jobs::tests::support::{public_ip, world_with};

#[tokio::test]
async fn should_hire_a_rented_servers_admin_who_intercepts_while_alive() {
    // Arrange
    let (mut world, network) = world_with(NetworkKind::Supermarket).await;
    let address = public_ip(&world, network).await;

    world
        .set_network_kind(network, NetworkKind::RentServer)
        .await
        .unwrap();

    // Act
    let none = world.fire_rental_admin(&address).await.unwrap();
    let without = world.has_super_admin(network).await.unwrap();
    let admin = world
        .hire_rental_admin(&address)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let with = world.has_super_admin(network).await.unwrap();
    let twice = world.hire_rental_admin(&address).await.unwrap();
    let row = world.db.npc(admin).await.unwrap().unwrap();

    world.set_dead(&row).await.unwrap();

    let after_death = world.has_super_admin(network).await.unwrap();

    // Hiring again removes the dead administrator and their charge (`NetworkLan.RemoveInvalidSuperAdminRental`,
    // `BankSubs.RemoveAdminPayment`), then charges the new one's first month and registers its charge.
    world.deposit("test", 3000.0).await.unwrap();

    let again = world.hire_rental_admin(&address).await.unwrap();
    let charge = world
        .bank_subscription("test", SubscriptionKind::Admin)
        .await
        .unwrap()
        .map(|subscription| subscription.amount);
    let fired = world.fire_rental_admin(&address).await.unwrap();

    // Assert
    assert_eq!(none, Some(Err(RentalAdminFailure::NoAdmin)));
    assert!(!without);
    assert!(with);
    assert_eq!(twice, Some(Err(RentalAdminFailure::AdminExists)));
    assert!(!after_death);
    assert!(matches!(again, Some(Ok(_))));
    assert_eq!(charge, Some(RENTAL_ADMIN_PRICE));
    assert_eq!(fired, Some(Ok(())));
    assert!(
        world
            .bank_subscription("test", SubscriptionKind::Admin)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!world.has_super_admin(network).await.unwrap());
}
