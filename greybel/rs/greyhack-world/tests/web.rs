//! Saves and opens a world on `wasm32-unknown-unknown`, the browser's runtime.

#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]

use greyhack_world::{Database, PlayerOptions, World, WorldOptions};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn should_open_the_world_it_saved_on_the_web() {
    // Arrange
    let mut world = World::generate(&WorldOptions {
        seed: 7,
        networks: 2,
        player: PlayerOptions::default(),
    })
    .await
    .expect("a generated world");
    let database = Database::in_memory().await.expect("an in-memory database");

    // Act
    world.save_into(&database).await.expect("the world saves");

    let loaded = World::load(&database).await.expect("the world loads");

    // Assert
    assert_eq!(
        (loaded.seed, loaded.player(), loaded.date()),
        (world.seed, world.player(), world.date())
    );
    assert_eq!(
        loaded.public_addresses().await.unwrap(),
        world.public_addresses().await.unwrap()
    );
}
