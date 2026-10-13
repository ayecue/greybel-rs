use crate::systems::guilds::records::tests::support::{SHARES, network_of, world_with_companies};

#[tokio::test]
async fn should_find_the_majority_owner_only_above_half() {
    // Arrange
    let (mut world, addresses) = world_with_companies(1).await;
    let network = network_of(&world, &addresses[0]).await;

    world
        .set_company_shares(&addresses[0], "small", SHARES / 2)
        .await
        .unwrap();

    // Act
    let at_half = world.company_majority_owner(network).await.unwrap();

    world
        .set_company_shares(&addresses[0], "big", SHARES / 2 + 1)
        .await
        .unwrap();

    let above = world.company_majority_owner(network).await.unwrap();

    // Assert
    assert_eq!(at_half, None);

    let (owner, percent) = above.expect("an owner");

    assert_eq!(owner, "big");
    assert!(percent > 50.0);
}
