use std::rc::Rc;
use std::sync::atomic::Ordering;

use support::{TestServices, database, player};

use crate::{ComputerId, CoreModule, WorldRef};

#[path = "support/mod.rs"]
mod support;

#[tokio::test]
async fn should_load_the_state_a_commit_stored() {
    // Arrange
    let database = database().await;
    let services = Rc::new(TestServices::default());
    let world = WorldRef::on(database.clone(), 7, ComputerId(0), 100);
    let registry = Rc::new(greyhack_world_modules::Registry::new(world.clone()));
    let core = CoreModule::new(&world, &registry);

    support::bind(&registry, &services);

    world.world_mut().player_person = player(&database).await;
    world.world_mut().time_seconds = 160;

    // Act
    core.commit().await.unwrap();

    let (loaded, loaded_core, loaded_registry) = CoreModule::load(&database).await.unwrap();

    support::bind(&loaded_registry, &services);
    loaded_core.load_state().await.unwrap();

    // Assert
    assert_eq!(loaded.world().seed, 7);
    assert_eq!(loaded.world().time_seconds, 160);
    assert_eq!(loaded.world().player_person, world.world().player_person);
    assert_eq!(loaded.world_seed(), world.world_seed());
}

#[tokio::test]
async fn should_fail_to_load_a_database_without_a_world() {
    // Arrange
    let database = database().await;

    // Act
    let loaded = CoreModule::load(&database).await;

    // Assert
    assert!(loaded.is_err());
}

#[tokio::test]
async fn should_ask_the_generator_for_the_router_of_an_unknown_address() {
    // Arrange
    let database = database().await;
    let services = Rc::new(TestServices::default());
    let world = WorldRef::on(database, 1, ComputerId(0), 0);
    let registry = Rc::new(greyhack_world_modules::Registry::new(world.clone()));
    let core = CoreModule::new(&world, &registry);

    support::bind(&registry, &services);

    // Act
    let whois = core.networks.dns.whois("8.8.8.8").await.unwrap();

    // Assert
    assert_eq!(whois, None);
    assert_eq!(services.generated.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn should_report_no_events_for_a_new_world() {
    // Arrange
    let database = database().await;
    let world = WorldRef::on(database, 1, ComputerId(0), 0);

    // Act
    let events_empty = world.events().is_empty();
    let changed = world.world_mut().take_changed_computers();

    // Assert
    assert!(events_empty);
    assert!(changed.is_empty());
}
