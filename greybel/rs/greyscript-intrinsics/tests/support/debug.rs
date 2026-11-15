use greybel_test_support::future::block_on;
use greyhack_behaviour::{FileData, LibraryKind, PermissionSet};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_libraries::{DebugAccount, Library, NodeDraft};

use super::set_library;

pub fn debug_world() -> World {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");

    // Debug accounts reference the shared password table; register the fixture password first.
    block_on(
        world
            .modules
            .npcs
            .mail
            .add_mail_account("dbg@debug.local", "secret"),
    )
    .unwrap();
    block_on(
        world
            .modules
            .libraries
            .debug
            .save_debug_account(&DebugAccount::new("dbg", "secret")),
    )
    .expect("the debug account is saved");

    let player = world.handle.player();

    set_library(
        &world,
        player,
        "/lib/metaxploit.so",
        Library {
            kind: LibraryKind::Metaxploit,
            version: "1.0.0".to_owned(),
            zones: Vec::new(),
        },
    );

    block_on(async {
        let root = world
            .modules
            .core
            .files
            .lookup
            .folder(player, "/root", true)
            .await
            .expect("the world reads")
            .expect("/root");

        let challenge = world
            .modules
            .libraries
            .debug
            .debug_library(player, "/lib/metaxploit.so")
            .await
            .expect("the world reads")
            .expect("the debug library")
            .challenge();

        world
            .modules
            .libraries
            .generator
            .add_full_node(
                &root,
                &NodeDraft::file(
                    "patch.src",
                    "test",
                    PermissionSet::default_for(false),
                    FileData::text(&challenge.correct_code),
                ),
            )
            .await
            .expect("the patch is added");
    });

    world
}
