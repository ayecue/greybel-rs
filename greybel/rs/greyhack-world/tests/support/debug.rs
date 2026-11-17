use greyhack_behaviour::{FileData, PermissionSet, ROOT_USER};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::ComputerId;
use greyhack_world_libraries::{NodeDraft, NpcDebugCredentials, ZeroDayRumor};

pub const LIBRARY_PATH: &str = "/lib/metaxploit.so";

pub async fn debug_world() -> (World, ComputerId) {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();

    world
        .modules
        .core
        .computers
        .users
        .register_password("secret")
        .await
        .unwrap();
    world
        .modules
        .libraries
        .debug
        .restore_rumor(&ZeroDayRumor {
            npc_passwords: [(
                "dbg".to_owned(),
                NpcDebugCredentials {
                    password: "secret".to_owned(),
                    player_id: None,
                },
            )]
            .into_iter()
            .collect(),
            ..ZeroDayRumor::default()
        })
        .await
        .unwrap();

    let root = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/root", true)
        .await
        .unwrap()
        .unwrap();

    let challenge = world
        .modules
        .libraries
        .debug
        .debug_library(computer, LIBRARY_PATH)
        .await
        .unwrap()
        .unwrap()
        .challenge();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &root,
            &NodeDraft::file(
                "patch.src",
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::text(&challenge.correct_code),
            ),
        )
        .await
        .unwrap();

    (world, computer)
}
