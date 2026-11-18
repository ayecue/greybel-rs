//! Opt-in timings for the expensive initial library history in world construction.

use std::time::Instant;

use greyhack_behaviour::{FIRST_VERSION, LibraryKind, Rng};
use greyhack_world::{DatabaseOpen, PlayerOptions, World, WorldOptions};
use greyhack_world_libraries::{Library, LibraryGeneration};

#[test]
#[ignore = "prints world-generation timings"]
fn world_generation_timing() {
    greybel_test_support::future::block_on(async {
        for run in 0..3 {
            let started = Instant::now();
            let world = World::new(&PlayerOptions::default()).await.unwrap();
            let history = world
                .modules
                .libraries
                .versions
                .saved_library_versions(greyhack_behaviour::LibraryKind::Metaxploit)
                .await
                .unwrap();
            println!(
                "World::new run {run}: {:?} ({} metaxploit versions)",
                started.elapsed(),
                history.len()
            );
        }

        let started = Instant::now();
        for kind in LibraryKind::ALL.iter().copied() {
            let mut rng = Rng::new(
                greyhack_behaviour::WorldSeed::new(Vec::new())
                    .derive_u64("library-history-v1", &[&kind.code().to_string()]),
            );
            std::hint::black_box(Library::generate(&mut rng, kind, FIRST_VERSION));
        }
        println!(
            "v1 library weakness/zone generation: {:?}",
            started.elapsed()
        );

        let started = Instant::now();
        let db = greyhack_world_core::Database::in_memory().await.unwrap();
        println!("database + schema: {:?}", started.elapsed());

        let started = Instant::now();
        let world = World::on(db, 0, greyhack_world_core::ComputerId(0), 0);
        println!("service assembly: {:?}", started.elapsed());

        world.begin().await.unwrap();
        let started = Instant::now();
        world
            .modules
            .libraries
            .versions
            .prepare_initial_versions()
            .await
            .unwrap();
        println!("initial library history: {:?}", started.elapsed());

        let started = Instant::now();
        world.database().commit().await.unwrap();
        println!("commit: {:?}", started.elapsed());

        let started = Instant::now();
        World::generate(&WorldOptions {
            networks: 1,
            ..WorldOptions::default()
        })
        .await
        .unwrap();
        println!(
            "World::generate with one extra network: {:?}",
            started.elapsed()
        );
    });
}
