use crate::systems::guilds::records::tests::support::{
    ADDRESS, LIBRARY_PATH, UNSEC_VALUE, crown, guild, has_crown, install, world_with_companies,
};
use crate::{CrownShieldFailure, ExploitLog};

#[tokio::test]
async fn should_protect_the_network_a_guild_hired_crownshield_for() {
    // Arrange
    let (mut world, addresses) = world_with_companies(2).await;

    crown(&mut world, "Owls", "owls-bank", &addresses[0]).await;

    // Act
    let before = world
        .is_crown_shield_protected(&addresses[1])
        .await
        .unwrap();
    let hired = world
        .hire_crown_shield("Owls", &addresses[1])
        .await
        .unwrap();
    let again = world
        .hire_crown_shield("Owls", &addresses[0])
        .await
        .unwrap();

    // Assert
    assert!(!before);
    assert_eq!(hired, Ok(()));
    assert!(
        world
            .is_crown_shield_protected(&addresses[1])
            .await
            .unwrap()
    );
    assert!(
        !world
            .is_crown_shield_protected(&addresses[0])
            .await
            .unwrap()
    );
    assert_eq!(again, Err(CrownShieldFailure::AlreadyActive));
}

#[tokio::test]
async fn should_refuse_crownshield_without_the_crown_or_for_a_bad_address() {
    // Arrange
    let (mut world, addresses) = world_with_companies(1).await;

    guild(&mut world, "Owls", "owls-bank", true).await;

    // Act
    let without = world
        .hire_crown_shield("Owls", &addresses[0])
        .await
        .unwrap();
    let lan = world
        .hire_crown_shield("Owls", "192.168.0.1")
        .await
        .unwrap();
    let unknown = world
        .hire_crown_shield("Nobody", &addresses[0])
        .await
        .unwrap();

    // Assert
    assert_eq!(without, Err(CrownShieldFailure::NoCrown));
    assert_eq!(lan, Err(CrownShieldFailure::InvalidAddress));
    assert_eq!(
        unknown,
        Err(CrownShieldFailure::UnknownGuild("Nobody".to_owned()))
    );
}

#[tokio::test]
async fn should_end_crownshield_when_the_guild_loses_the_crown() {
    // Arrange
    let (mut world, addresses) = world_with_companies(2).await;

    crown(&mut world, "Owls", "owls-bank", &addresses[0]).await;
    world
        .hire_crown_shield("Owls", &addresses[1])
        .await
        .unwrap()
        .unwrap();

    // Act
    world
        .set_company_shares(&addresses[0], "owls-bank", 0)
        .await
        .unwrap();
    world.tick(60).await.unwrap();

    // Assert
    assert!(!has_crown(&world, "Owls").await);
    assert!(
        !world
            .is_crown_shield_protected(&addresses[1])
            .await
            .unwrap()
    );
    assert!(
        world
            .guilds()
            .await
            .unwrap()
            .iter()
            .all(|guild| guild.crown_shield_public_ip.is_empty())
    );
}

#[tokio::test]
async fn should_restart_a_network_that_gets_crownshield() {
    // Arrange
    let (mut world, addresses) = world_with_companies(2).await;

    crown(&mut world, "Owls", "owls-bank", &addresses[0]).await;

    let router = world.router_at(&addresses[1]).await.unwrap().unwrap();

    // Act
    world
        .hire_crown_shield("Owls", &addresses[1])
        .await
        .unwrap()
        .unwrap();

    // Assert
    let row = world.computer(router.id.computer()).await.unwrap().unwrap();

    assert!(row.is_rebooting(world.time_seconds()));
}

#[tokio::test]
async fn should_interrupt_an_overflow_against_a_crownshield_network() {
    // Arrange
    let (mut world, addresses) = world_with_companies(1).await;
    let computer = world.player();
    let home = world.public_ip(computer).await.unwrap();

    crown(&mut world, "Owls", "owls-bank", &addresses[0]).await;
    install(&world, computer).await;

    let free = world
        .overflow_file(computer, LIBRARY_PATH, ADDRESS, UNSEC_VALUE, "")
        .await
        .unwrap();

    world
        .hire_crown_shield("Owls", &home)
        .await
        .unwrap()
        .unwrap();

    // Act
    let protected = world
        .overflow_file(computer, LIBRARY_PATH, ADDRESS, UNSEC_VALUE, "")
        .await
        .unwrap();
    let missing = world
        .overflow_file(computer, LIBRARY_PATH, "0x0", UNSEC_VALUE, "")
        .await
        .unwrap();

    // Assert
    assert_ne!(free.log, ExploitLog::CrownShield);
    assert_eq!(protected.log, ExploitLog::CrownShield);
    assert_eq!(missing.log, ExploitLog::MissingWeakness);
}
