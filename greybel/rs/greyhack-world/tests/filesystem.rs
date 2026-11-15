use greyhack_behaviour::{
    FileData, MAX_FILES, MAX_FILES_PER_FOLDER, Permission, PermissionSet, ROOT_USER, parse_date,
};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{FileFailure, NodeDraft};
use greyhack_world_services::AptFailure;

use crate::support::filesystem::{add_source_file, computers_for_copy, tree};

#[tokio::test]
async fn should_print_the_paths_of_found_nodes() {
    let (world, computer) = tree().await;
    let user = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", true)
        .await
        .unwrap()
        .unwrap();
    let notes = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/home/bob/notes.txt", true)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(user.path, "/home/bob");
    assert_eq!(notes.path, "/home/bob/notes.txt");
}

#[tokio::test]
async fn should_find_folders_by_path() {
    let (world, computer) = tree().await;
    let user = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", true)
        .await
        .unwrap()
        .unwrap();
    let lookups = [
        ("home/bob", true),
        ("/home/bob/", true),
        ("/nope", true),
        ("/link", true),
        ("/link", false),
    ];
    let mut found = Vec::new();

    for (path, follow) in lookups {
        found.push(
            world
                .modules
                .core
                .files
                .lookup
                .folder(computer, path, follow)
                .await
                .unwrap(),
        );
    }

    let [
        relative,
        trailing_slash,
        missing,
        followed_link,
        unfollowed_link,
    ]: [Option<_>; 5] = found.try_into().unwrap();

    assert_eq!(relative, Some(user.clone()));
    assert_eq!(trailing_slash, Some(user.clone()));
    assert_eq!(missing, None);
    assert_eq!(followed_link, Some(user.clone()));
    assert_ne!(unfollowed_link, Some(user));
}

#[tokio::test]
async fn should_find_an_entry_by_path() {
    let (world, computer) = tree().await;
    let user = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", true)
        .await
        .unwrap()
        .unwrap();
    let entry = world
        .modules
        .core
        .files
        .lookup
        .entry(computer, "/home/bob", true)
        .await
        .unwrap();

    assert_eq!(entry, Some(user));
}

#[tokio::test]
async fn should_find_a_file_through_a_symlink() {
    let (world, computer) = tree().await;
    let notes = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/home/bob/notes.txt", true)
        .await
        .unwrap()
        .unwrap();
    let found = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/link/notes.txt", true)
        .await
        .unwrap();

    assert_eq!(found, Some(notes));
}

#[tokio::test]
async fn should_replace_a_file_when_adding_a_folder_of_its_name() {
    let (world, computer) = tree().await;
    let user = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", true)
        .await
        .unwrap()
        .unwrap();
    let before = world
        .modules
        .core
        .files
        .sizes
        .file_count(computer)
        .await
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &user,
            &NodeDraft::folder("notes.txt", "bob", PermissionSet::default_for(true)),
        )
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .files(&user)
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .folders(&user)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .sizes
            .file_count(computer)
            .await
            .unwrap(),
        before
    );
}

#[tokio::test]
async fn should_free_the_nodes_of_a_removed_folder() {
    let (world, computer) = tree().await;
    let user = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", true)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &user,
            &NodeDraft::folder("notes.txt", "bob", PermissionSet::default_for(true)),
        )
        .await
        .unwrap();

    let home = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home", true)
        .await
        .unwrap()
        .unwrap();
    let before = world
        .modules
        .core
        .files
        .sizes
        .file_count(computer)
        .await
        .unwrap();
    let removed = world
        .modules
        .core
        .files
        .sizes
        .descendant_count(&home)
        .await
        .unwrap()
        + 1;

    world
        .modules
        .core
        .files
        .changes
        .remove_node(&home)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .files
            .lookup
            .folder(computer, "/home/bob", true)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .sizes
            .file_count(computer)
            .await
            .unwrap(),
        before - removed
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .folder(computer, "/link", false)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_copy_a_folder() {
    let (world, computer) = tree().await;
    let user = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", true)
        .await
        .unwrap()
        .unwrap();
    let root = world
        .modules
        .core
        .files
        .lookup
        .root(computer)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .files
        .changes
        .copy_node(&user, &root, "copy")
        .await
        .unwrap();

    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(computer, "/copy/notes.txt", true)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(computer, "/home/bob/notes.txt", true)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_move_a_folder() {
    let (world, computer) = tree().await;
    let user = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", true)
        .await
        .unwrap()
        .unwrap();
    let root = world
        .modules
        .core
        .files
        .lookup
        .root(computer)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .files
        .changes
        .copy_node(&user, &root, "copy")
        .await
        .unwrap();

    let copy = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/copy", true)
        .await
        .unwrap()
        .unwrap();
    let tmp = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &root,
            &NodeDraft::folder("tmp", "root", PermissionSet::default_for(true)),
        )
        .await
        .unwrap();

    world
        .modules
        .core
        .files
        .changes
        .move_node(&copy, &tmp, "moved")
        .await
        .unwrap();

    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .folder(computer, "/copy", true)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(computer, "/tmp/moved/notes.txt", true)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_reject_a_copy_that_exceeds_the_destination_free_space() {
    let (mut world, source, destination) = computers_for_copy().await;

    add_source_file(&mut world, source, FileData::binary(10_000_000_001)).await;

    let source_node = world
        .modules
        .core
        .files
        .lookup
        .file(source, "/source.txt", false)
        .await
        .unwrap()
        .unwrap();
    let source_size = world
        .modules
        .core
        .files
        .sizes
        .size(&source_node, false)
        .await
        .unwrap();
    let free = world
        .modules
        .core
        .files
        .sizes
        .available_storage_mb(destination)
        .await
        .unwrap();

    assert_eq!(source_size, 10_000_000_001);
    assert!(10_000.000_001 > free, "destination free space: {free} MB");

    let result = world
        .modules
        .core
        .files
        .changes
        .copy_between(
            (source, ROOT_USER),
            "/source.txt",
            (destination, ROOT_USER),
            "/",
        )
        .await
        .unwrap();

    assert_eq!(result, Err(FileFailure::DiskFull));
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(destination, "/source.txt", false)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn should_count_destination_folders_toward_the_copy_entry_limit() {
    let (mut world, source, destination) = computers_for_copy().await;

    add_source_file(&mut world, source, FileData::text("source")).await;

    let target = world
        .modules
        .core
        .files
        .lookup
        .root(destination)
        .await
        .unwrap()
        .unwrap();

    for index in 0..=MAX_FILES_PER_FOLDER {
        world
            .modules
            .libraries
            .generator
            .add_full_node(
                &target,
                &NodeDraft::folder(
                    &format!("folder-{index}"),
                    ROOT_USER,
                    PermissionSet::default_for(true),
                ),
            )
            .await
            .unwrap();
    }

    let result = world
        .modules
        .core
        .files
        .changes
        .copy_between(
            (source, ROOT_USER),
            "/source.txt",
            (destination, ROOT_USER),
            "/",
        )
        .await
        .unwrap();

    assert_eq!(result, Err(FileFailure::FolderFull));
}

#[tokio::test]
async fn should_reject_a_copy_that_exceeds_the_destination_global_node_limit() {
    let (mut world, source, destination) = computers_for_copy().await;

    add_source_file(&mut world, source, FileData::text("source")).await;

    let target = world
        .modules
        .core
        .files
        .lookup
        .root(destination)
        .await
        .unwrap()
        .unwrap();
    let filler = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &target,
            &NodeDraft::folder("filler", ROOT_USER, PermissionSet::default_for(true)),
        )
        .await
        .unwrap();
    let mut index = 0;

    while world
        .modules
        .core
        .files
        .sizes
        .file_count(destination)
        .await
        .unwrap()
        < MAX_FILES
    {
        world
            .modules
            .libraries
            .generator
            .add_full_node(
                &filler,
                &NodeDraft::file(
                    &format!("file-{index}"),
                    ROOT_USER,
                    PermissionSet::default_for(false),
                    FileData::default(),
                ),
            )
            .await
            .unwrap();
        index += 1;
    }

    let result = world
        .modules
        .core
        .files
        .changes
        .copy_between(
            (source, ROOT_USER),
            "/source.txt",
            (destination, ROOT_USER),
            "/",
        )
        .await
        .unwrap();

    assert_eq!(result, Err(FileFailure::TooManyFiles));
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(destination, "/source.txt", false)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn should_require_an_execute_bit_for_root_and_for_traversal() {
    let (world, computer) = tree().await;
    let root_user = world
        .modules
        .core
        .computers
        .users
        .user(computer, ROOT_USER)
        .await
        .unwrap()
        .unwrap();
    let test_user = world
        .modules
        .core
        .computers
        .users
        .user(computer, "test")
        .await
        .unwrap()
        .unwrap();
    let mut notes = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/home/bob/notes.txt", false)
        .await
        .unwrap()
        .unwrap();

    notes.permissions = PermissionSet::parse("-rw-------").unwrap();
    assert!(!notes.can_execute(Some(&root_user)));
    notes.permissions = PermissionSet::parse("-rw------x").unwrap();
    assert!(notes.can_execute(Some(&root_user)));

    notes.permissions = PermissionSet::default_for(false);
    world
        .modules
        .core
        .files
        .changes
        .save_node(&notes)
        .await
        .unwrap();

    let mut bob = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", false)
        .await
        .unwrap()
        .unwrap();

    bob.permissions = PermissionSet::parse("drwx------").unwrap();
    world
        .modules
        .core
        .files
        .changes
        .save_node(&bob)
        .await
        .unwrap();

    assert!(
        !world
            .modules
            .core
            .files
            .permissions
            .has_node_permission(&notes, Permission::Read, Some(&test_user))
            .await
            .unwrap()
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .entry_for_user(computer, "/home/bob/notes.txt", false, "test")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .entry_for_user(computer, "/home/bob/notes.txt", false, ROOT_USER)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_require_write_and_execute_to_modify_folder_contents() {
    let (world, computer) = tree().await;
    let account = world
        .modules
        .core
        .computers
        .users
        .user(computer, "test")
        .await
        .unwrap()
        .unwrap();
    let mut home = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/test", false)
        .await
        .unwrap()
        .unwrap();

    home.permissions = PermissionSet::parse("drw--wx-wx").unwrap();
    world
        .modules
        .core
        .files
        .changes
        .save_node(&home)
        .await
        .unwrap();
    assert!(
        !world
            .modules
            .core
            .files
            .permissions
            .can_modify_contents(&home, Some(&account))
            .await
            .unwrap()
    );

    home.permissions = PermissionSet::parse("drwxrwxrwx").unwrap();
    world
        .modules
        .core
        .files
        .changes
        .save_node(&home)
        .await
        .unwrap();
    assert!(
        world
            .modules
            .core
            .files
            .permissions
            .can_modify_contents(&home, Some(&account))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn should_check_owner_and_group_changes_recursively() {
    let (world, computer) = tree().await;
    let account = world
        .modules
        .core
        .computers
        .users
        .user(computer, "test")
        .await
        .unwrap()
        .unwrap();
    let guest = world
        .modules
        .core
        .computers
        .users
        .user(computer, "guest")
        .await
        .unwrap()
        .unwrap();
    let home = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/test", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .files
            .permissions
            .set_owner(&home, Some(ROOT_USER), None, Some(&account), false)
            .await
            .unwrap(),
        Err(FileFailure::NoPermission)
    );
    assert_eq!(
        world
            .modules
            .core
            .files
            .permissions
            .set_owner(&home, Some("test"), None, Some(&guest), false)
            .await
            .unwrap(),
        Err(FileFailure::NoPermission)
    );

    let child = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &home,
            &NodeDraft::file(
                "root.txt",
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::text("root"),
            ),
        )
        .await
        .unwrap();

    world
        .modules
        .core
        .files
        .permissions
        .set_owner(&home, Some("test"), Some("test"), Some(&account), true)
        .await
        .unwrap()
        .unwrap();

    let child = world
        .modules
        .core
        .files
        .lookup
        .node(computer, child.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(child.owner, ROOT_USER);
}

#[tokio::test]
async fn should_merge_copies_and_preserve_destination_file_metadata() {
    let (world, computer) = tree().await;
    let account = world
        .modules
        .core
        .computers
        .users
        .user(computer, ROOT_USER)
        .await
        .unwrap()
        .unwrap();
    let source = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", false)
        .await
        .unwrap()
        .unwrap();
    let root = world
        .modules
        .core
        .files
        .lookup
        .root(computer)
        .await
        .unwrap()
        .unwrap();

    let first = world
        .modules
        .core
        .files
        .changes
        .copy_with_rules(
            &source,
            &root,
            "copy",
            Some(&account),
            Some(&account),
            false,
        )
        .await
        .unwrap()
        .unwrap();
    let extra = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &first.node,
            &NodeDraft::file(
                "extra.txt",
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::text("extra"),
            ),
        )
        .await
        .unwrap();
    let mut notes = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/copy/notes.txt", false)
        .await
        .unwrap()
        .unwrap();

    notes.owner = "test".to_owned();
    notes.group = "test".to_owned();
    notes.permissions = PermissionSet::parse("-rw-r--r--").unwrap();
    world
        .modules
        .core
        .files
        .changes
        .save_node(&notes)
        .await
        .unwrap();

    let second = world
        .modules
        .core
        .files
        .changes
        .copy_with_rules(
            &source,
            &root,
            "copy",
            Some(&account),
            Some(&account),
            false,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(second.node.id, first.node.id);
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .node(computer, extra.id)
            .await
            .unwrap()
            .is_some()
    );

    let notes = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/copy/notes.txt", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        (notes.owner.as_str(), notes.permissions.as_str()),
        ("test", "-rw-r--r--")
    );
}

#[tokio::test]
async fn should_skip_unreadable_copy_entries_and_reject_overwriting_oneself() {
    let (world, computer) = tree().await;
    let account = world
        .modules
        .core
        .computers
        .users
        .user(computer, "test")
        .await
        .unwrap()
        .unwrap();
    let bob = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/bob", false)
        .await
        .unwrap()
        .unwrap();
    let home = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/test", false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &bob,
            &NodeDraft::file(
                "secret.txt",
                ROOT_USER,
                PermissionSet::parse("-rw-------").unwrap(),
                FileData::text("secret"),
            ),
        )
        .await
        .unwrap();

    let report = world
        .modules
        .core
        .files
        .changes
        .copy_with_rules(
            &bob,
            &home,
            "filtered",
            Some(&account),
            Some(&account),
            false,
        )
        .await
        .unwrap()
        .unwrap();

    assert!(report.skipped);
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(computer, "/home/test/filtered/notes.txt", false)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .file(computer, "/home/test/filtered/secret.txt", false)
            .await
            .unwrap()
            .is_none()
    );

    let notes = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/home/bob/notes.txt", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .files
            .changes
            .copy_with_rules(
                &notes,
                &bob,
                "notes.txt",
                Some(&account),
                Some(&account),
                false
            )
            .await
            .unwrap(),
        Err(FileFailure::OverwriteSelf)
    );
}

#[tokio::test]
async fn should_delete_depth_first_and_report_partial_or_cancelled_deletion() {
    let (world, computer) = tree().await;
    let account = world
        .modules
        .core
        .computers
        .users
        .user(computer, "test")
        .await
        .unwrap()
        .unwrap();
    let home = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/test", false)
        .await
        .unwrap()
        .unwrap();
    let top = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &home,
            &NodeDraft::folder("top", "test", PermissionSet::default_for(true)),
        )
        .await
        .unwrap();
    let locked = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &top,
            &NodeDraft::folder("locked", ROOT_USER, PermissionSet::default_for(true)),
        )
        .await
        .unwrap();
    let keep = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &top,
            &NodeDraft::file(
                "keep.txt",
                "test",
                PermissionSet::default_for(false),
                FileData::text("keep"),
            ),
        )
        .await
        .unwrap();
    let secret = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &locked,
            &NodeDraft::file(
                "secret.txt",
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::text("secret"),
            ),
        )
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .core
            .files
            .changes
            .delete_with_rules(&top, Some(&account))
            .await
            .unwrap(),
        Ok(true)
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .node(computer, keep.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .node(computer, secret.id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .node(computer, top.id)
            .await
            .unwrap()
            .is_some()
    );

    assert_eq!(
        world
            .modules
            .core
            .files
            .changes
            .delete_with_rules(&locked, Some(&account))
            .await
            .unwrap(),
        Err(FileFailure::DeletePermission(locked.path.clone()))
    );
    assert!(
        world
            .modules
            .core
            .files
            .lookup
            .node(computer, secret.id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn should_timestamp_new_nodes_and_keep_timestamps_on_copies() {
    let (world, computer) = tree().await;
    let account = world
        .modules
        .core
        .computers
        .users
        .user(computer, ROOT_USER)
        .await
        .unwrap()
        .unwrap();

    let root = world
        .modules
        .core
        .files
        .lookup
        .root(computer)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(root.permissions.as_str(), "drwxr-xr-x");
    assert!(root.timestamp < parse_date(greyhack_behaviour::START_DATE));
    world.modules.routines.set_date("05/Mar/2026 - 14:07");

    let home = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/home/test", false)
        .await
        .unwrap()
        .unwrap();
    let file = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &home,
            &NodeDraft::file(
                "dated",
                "test",
                PermissionSet::default_for(false),
                FileData::text("date"),
            ),
        )
        .await
        .unwrap();

    assert_eq!(file.timestamp, parse_date("05/Mar/2026 - 14:07"));

    let copy = world
        .modules
        .core
        .files
        .changes
        .copy_with_rules(&file, &home, "copy", Some(&account), Some(&account), false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(copy.node.timestamp, file.timestamp);
}

#[tokio::test]
async fn should_transfer_ownership_and_preserve_metadata_on_scp_overwrite() {
    let (mut world, source, _) = computers_for_copy().await;
    let destination = world.handle.player();

    add_source_file(&mut world, source, FileData::text("scp")).await;

    let report = world
        .modules
        .core
        .files
        .changes
        .copy_between(
            (source, ROOT_USER),
            "/source.txt",
            (destination, "test"),
            "/home/test",
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(report.node.owner, "test");

    let mut destination_node = report.node;

    destination_node.group = "custom".to_owned();
    destination_node.permissions = PermissionSet::parse("-rwxr-xr-x").unwrap();
    world
        .modules
        .core
        .files
        .changes
        .save_node(&destination_node)
        .await
        .unwrap();
    world
        .modules
        .core
        .files
        .changes
        .copy_between(
            (source, ROOT_USER),
            "/source.txt",
            (destination, "test"),
            "/home/test",
        )
        .await
        .unwrap()
        .unwrap();

    let copied = world
        .modules
        .core
        .files
        .lookup
        .file(destination, "/home/test/source.txt", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(copied.owner, "test");
    assert_eq!(copied.group, "custom");
    assert_eq!(copied.permissions.as_str(), "-rwxr-xr-x");
}

#[tokio::test]
async fn should_store_raw_commands_and_validate_the_apt_cache() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let computer = world.handle.player();
    let root = world
        .modules
        .core
        .files
        .lookup
        .root(computer)
        .await
        .unwrap()
        .unwrap();
    let binary = world
        .modules
        .libraries
        .generator
        .add_full_node(
            &root,
            &NodeDraft::file(
                "record.bin",
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::binary(42),
            ),
        )
        .await
        .unwrap();

    assert_eq!(binary.data().unwrap().command_name, "record.bin");
    world
        .modules
        .services
        .apt
        .update_apt_packages(computer, &[])
        .await
        .unwrap();
    assert_eq!(
        world
            .modules
            .services
            .apt
            .validate_apt_cache(computer)
            .await
            .unwrap(),
        Ok(())
    );

    let cache = world
        .modules
        .core
        .files
        .lookup
        .file(computer, "/etc/apt/aptcache.bin", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(cache.data().unwrap().command_name, "aptcache.bin");

    let apt = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, "/etc/apt", false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &apt,
            &NodeDraft::file(
                "aptcache.bin",
                ROOT_USER,
                PermissionSet::default_for(false),
                FileData::text("not a cache"),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        world
            .modules
            .services
            .apt
            .validate_apt_cache(computer)
            .await
            .unwrap(),
        Err(AptFailure::InvalidCache)
    );
}
