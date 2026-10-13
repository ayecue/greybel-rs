use greyhack_behaviour::{FileData, PermissionSet, ROOT_USER};
use greyhack_world::{ComputerId, DebugAccount, NodeDraft, PlayerOptions, World};

pub const LIBRARY_PATH: &str = "/lib/metaxploit.so";

pub async fn debug_world() -> (World, ComputerId) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.player();

    world.register_password("secret").await.unwrap();
    world
        .save_debug_account(&DebugAccount::new("dbg", "secret"))
        .await
        .unwrap();

    let root = world
        .folder(computer, "/root", true)
        .await
        .unwrap()
        .unwrap();

    let challenge = world
        .debug_library(computer, LIBRARY_PATH)
        .await
        .unwrap()
        .unwrap()
        .challenge();

    world
        .add_node(
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
