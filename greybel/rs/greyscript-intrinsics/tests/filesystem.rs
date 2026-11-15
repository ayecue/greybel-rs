use greyhack_behaviour::{FileData, PermissionSet, ROOT_USER, parse_date};
use greyhack_world_core::NodeDraft;

use crate::support::filesystem::world;
use crate::support::scripts::run_on_host;

#[test]
fn should_print_file_dates_without_following_symlinks() {
    let world = world();
    let computer = world.handle.player();

    world.modules.routines.set_date("05/Mar/2026 - 14:07");

    let home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        computer,
        "/home/test",
        false,
    ))
    .unwrap()
    .unwrap();

    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &home,
        &NodeDraft::file(
            "dated",
            "test",
            PermissionSet::default_for(false),
            FileData::text("date"),
        ),
    ))
    .unwrap();

    let (output, error, world) = run_on_host(
        "print get_shell.host_computer.File(home_dir + \"/dated\").date",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(output.trim_end(), "05/Mar/2026 14:07");

    let file = greybel_test_support::future::block_on(world.modules.core.files.lookup.file(
        computer,
        "/home/test/dated",
        false,
    ))
    .unwrap()
    .unwrap();

    assert_eq!(file.timestamp, parse_date("05/Mar/2026 - 14:07"));
}

#[test]
fn should_check_write_on_the_file_for_set_content_and_update_size_and_date() {
    let world = world();
    let computer = world.handle.player();
    let mut home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        computer,
        "/home/test",
        false,
    ))
    .unwrap()
    .unwrap();
    let mut file =
        greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
            &home,
            &NodeDraft::file(
                "content",
                "test",
                PermissionSet::parse("-rw-r--r--").unwrap(),
                FileData::text(""),
            ),
        ))
        .unwrap();

    home.permissions = PermissionSet::parse("drwxr-xr-x").unwrap();
    file.timestamp = 0;
    greybel_test_support::future::block_on(world.modules.core.files.changes.save_node(&home))
        .unwrap();
    greybel_test_support::future::block_on(world.modules.core.files.changes.save_node(&file))
        .unwrap();
    world.modules.routines.set_date("05/Mar/2026 - 14:07");

    let (output, error, _world) = run_on_host(
        "f = get_shell.host_computer.File(home_dir + \"/content\")\nprint f.set_content(\"a😀b\")\nprint f.size\nprint f.date",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(
        output.lines().collect::<Vec<_>>(),
        ["1", "4", "05/Mar/2026 14:07"]
    );
}

#[test]
fn should_word_listing_and_self_copy_failures_like_the_game() {
    let world = world();
    let computer = world.handle.player();
    let mut etc = greybel_test_support::future::block_on(
        world
            .modules
            .core
            .files
            .lookup
            .folder(computer, "/etc", false),
    )
    .unwrap()
    .unwrap();

    etc.permissions = PermissionSet::parse("drwx------").unwrap();
    greybel_test_support::future::block_on(world.modules.core.files.changes.save_node(&etc))
        .unwrap();

    let (output, error, _) = run_on_host(
        "pc = get_shell.host_computer\npc.touch(home_dir, \"same\")\nprint pc.File(\"/etc\").get_folders\nprint pc.File(home_dir + \"/same\").copy(home_dir, \"same\")",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(
        output.lines().collect::<Vec<_>>(),
        ["Permission denied: /etc", "A file cannot overwrite itself"]
    );
}

#[test]
fn should_return_null_when_an_intrinsic_cannot_traverse_the_file() {
    let world = world();
    let computer = world.handle.player();
    let home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        computer,
        "/home/test",
        false,
    ))
    .unwrap()
    .unwrap();
    let locked =
        greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
            &home,
            &NodeDraft::folder(
                "locked",
                ROOT_USER,
                PermissionSet::parse("drwx------").unwrap(),
            ),
        ))
        .unwrap();

    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &locked,
        &NodeDraft::file(
            "secret",
            ROOT_USER,
            PermissionSet::default_for(false),
            FileData::text("secret"),
        ),
    ))
    .unwrap();

    let (output, error, _) = run_on_host(
        "print get_shell.host_computer.File(home_dir + \"/locked/secret\").get_content",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(output.trim_end(), "null");
}

#[test]
fn should_warn_when_copy_and_delete_skip_entries() {
    let world = world();
    let computer = world.handle.player();
    let home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        computer,
        "/home/test",
        false,
    ))
    .unwrap()
    .unwrap();
    let source =
        greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
            &home,
            &NodeDraft::folder("source", "test", PermissionSet::default_for(true)),
        ))
        .unwrap();

    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &source,
        &NodeDraft::file(
            "visible",
            "test",
            PermissionSet::default_for(false),
            FileData::text("visible"),
        ),
    ))
    .unwrap();
    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &source,
        &NodeDraft::file(
            "secret",
            ROOT_USER,
            PermissionSet::parse("-rw-------").unwrap(),
            FileData::text("secret"),
        ),
    ))
    .unwrap();

    let (output, error, world) = run_on_host(
        "print get_shell.host_computer.File(home_dir + \"/source\").copy(home_dir, \"copy\")",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(
        output.trim_end(),
        "Warning: one or more files or folders cannot be copied."
    );

    let home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        computer,
        "/home/test",
        false,
    ))
    .unwrap()
    .unwrap();
    let top =
        greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
            &home,
            &NodeDraft::folder("top", "test", PermissionSet::default_for(true)),
        ))
        .unwrap();
    let locked =
        greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
            &top,
            &NodeDraft::folder("locked", ROOT_USER, PermissionSet::default_for(true)),
        ))
        .unwrap();

    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &top,
        &NodeDraft::file(
            "removed",
            "test",
            PermissionSet::default_for(false),
            FileData::text("removed"),
        ),
    ))
    .unwrap();
    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &locked,
        &NodeDraft::file(
            "kept",
            ROOT_USER,
            PermissionSet::default_for(false),
            FileData::text("kept"),
        ),
    ))
    .unwrap();

    let (output, error, _) = run_on_host(
        "pc = get_shell.host_computer\nprint pc.File(home_dir + \"/top\").delete\nprint pc.File(home_dir + \"/top/locked\").delete",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(
        output.lines().collect::<Vec<_>>(),
        [
            "Warning: one or more files or folders cannot be deleted.",
            "Deletion cancelled: permission denied: /home/test/top/locked. No files or folders were deleted.",
        ]
    );
}

#[test]
fn should_move_without_read_but_require_folder_write_only_across_parents() {
    let world = world();
    let computer = world.handle.player();
    let home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        computer,
        "/home/test",
        false,
    ))
    .unwrap()
    .unwrap();
    let source =
        greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
            &home,
            &NodeDraft::folder("source", "test", PermissionSet::default_for(true)),
        ))
        .unwrap();

    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &source,
        &NodeDraft::file(
            "no-read",
            ROOT_USER,
            PermissionSet::parse("-rw-------").unwrap(),
            FileData::text("secret"),
        ),
    ))
    .unwrap();
    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &source,
        &NodeDraft::folder("locked", ROOT_USER, PermissionSet::default_for(true)),
    ))
    .unwrap();

    let (output, error, _) = run_on_host(
        "pc = get_shell.host_computer\nprint pc.File(home_dir + \"/source/no-read\").move(home_dir + \"/Downloads\", \"moved\")\nprint pc.File(home_dir + \"/source/locked\").move(home_dir + \"/Downloads\", \"locked\")\nprint pc.File(home_dir + \"/source/locked\").move(home_dir + \"/source\", \"renamed\")",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(
        output.lines().collect::<Vec<_>>(),
        ["1", "permission denied", "1"]
    );
}

#[test]
fn should_require_write_and_execute_on_the_parent_for_rename() {
    let world = world();
    let computer = world.handle.player();
    let mut home = greybel_test_support::future::block_on(world.modules.core.files.lookup.folder(
        computer,
        "/home/test",
        false,
    ))
    .unwrap()
    .unwrap();
    let file =
        greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
            &home,
            &NodeDraft::file(
                "old",
                "test",
                PermissionSet::default_for(false),
                FileData::text("old"),
            ),
        ))
        .unwrap();

    home.permissions = PermissionSet::parse("drw--wx-wx").unwrap();
    greybel_test_support::future::block_on(world.modules.core.files.changes.save_node(&home))
        .unwrap();
    greybel_test_support::future::block_on(world.modules.core.files.changes.save_node(&file))
        .unwrap();

    let (output, error, _) = run_on_host(
        "print get_shell.host_computer.File(home_dir + \"/old\").rename(\"new\")",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(output.trim_end(), "permission denied");
}

#[test]
fn should_reject_a_text_file_renamed_as_the_apt_cache() {
    let world = world();
    let computer = world.handle.player();
    let apt = greybel_test_support::future::block_on(
        world
            .modules
            .core
            .files
            .lookup
            .folder(computer, "/etc/apt", false),
    )
    .unwrap()
    .unwrap();

    greybel_test_support::future::block_on(world.modules.libraries.generator.add_full_node(
        &apt,
        &NodeDraft::file(
            "aptcache.bin",
            ROOT_USER,
            PermissionSet::default_for(false),
            FileData::text("not a cache"),
        ),
    ))
    .unwrap();

    let (output, error, _) = run_on_host(
        "print include_lib(\"/lib/aptclient.so\").search(\"crypto\")",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(
        output.trim_end(),
        "/etc/apt/aptcache.bin is not a valid apt cache. Launch apt with the update option to refresh the file"
    );
}
