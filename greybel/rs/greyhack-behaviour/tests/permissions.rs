use greyhack_behaviour::{Permission, PermissionSet, Principal, home_folders};

use crate::support::permissions::user;

#[test]
fn should_give_owner_group_and_others_each_their_triple() {
    let perms = PermissionSet::parse("-rw-r----x").unwrap();
    let owner = user("alice", &[]);
    let member = user("bob", &["staff"]);
    let other = user("carol", &[]);
    let root = user("root", &[]);
    let cases = [
        (Permission::Write, Some(&owner)),
        (Permission::Execute, Some(&owner)),
        (Permission::Read, Some(&member)),
        (Permission::Write, Some(&member)),
        (Permission::Execute, Some(&other)),
        (Permission::Read, Some(&other)),
        (Permission::Execute, Some(&root)),
        (Permission::Read, None),
    ];
    let [
        owner_writes,
        owner_executes,
        member_reads,
        member_writes,
        other_executes,
        other_reads,
        root_executes,
        nobody_reads,
    ] = cases.map(|(permission, user)| {
        perms.allows(
            permission,
            "alice",
            "staff",
            user.map(|user| Principal::new(&user.name, &user.groups)),
        )
    });

    assert!(owner_writes);
    assert!(!owner_executes);
    assert!(member_reads);
    assert!(!member_writes);
    assert!(other_executes);
    assert!(!other_reads);
    assert!(root_executes);
    assert!(!nobody_reads);
}

#[test]
fn should_change_like_chmod() {
    let perms = PermissionSet::default_for(true);
    let changes = ["o+w", "u-rwx", "g+", "ug+r", "u+rwxr", "a+r"];
    let [
        others_write,
        owner_none,
        nothing,
        two_classes,
        repeated,
        all,
    ] = changes.map(|change| perms.with_change(change));

    assert_eq!(others_write.unwrap().as_str(), "drwxr-xrwx");
    assert_eq!(owner_none.unwrap().as_str(), "d---r-xr-x");
    assert_eq!(nothing.unwrap(), perms);
    assert_eq!(two_classes, None);
    assert_eq!(repeated, None);
    assert_eq!(all, None);
}

#[test]
fn should_require_some_execute_bit_for_root_on_a_regular_file() {
    let root = user("root", &[]);
    let principal = Principal::new(&root.name, &root.groups);
    let without_execute = PermissionSet::parse("-rw-rw-rw-").unwrap();
    let other_execute = PermissionSet::parse("-rw-rw-rwx").unwrap();
    let folder = PermissionSet::parse("drw-rw-rw-").unwrap();

    assert!(!without_execute.allows(Permission::Execute, "alice", "staff", Some(principal)));
    assert!(other_execute.allows(Permission::Execute, "alice", "staff", Some(principal)));
    assert!(folder.allows(Permission::Execute, "alice", "staff", Some(principal)));
    assert!(without_execute.allows(Permission::Write, "alice", "staff", Some(principal)));
}

#[test]
fn should_share_home_folders_but_keep_the_trash_private() {
    let folders = home_folders(PermissionSet::GROUP_FOLDER);
    let names: Vec<&str> = folders.iter().map(|(name, _)| *name).collect();

    assert_eq!(names, ["Desktop", "Downloads", "Config", ".Trash"]);
    assert_eq!(folders[0].1.as_str(), "drwxrwx---");
    assert_eq!(folders[2].1.as_str(), "drwxrwxr-x");
    assert_eq!(folders[3].1.as_str(), "drwx------");
    assert_eq!(
        home_folders(PermissionSet::OPEN_FOLDER)[2].1.as_str(),
        "drwxrwxrwx"
    );
}

#[test]
fn should_read_a_single_letter_as_a_permission() {
    assert_eq!(Permission::from_mode("r"), Some(Permission::Read));
    assert_eq!(Permission::from_mode("w"), Some(Permission::Write));
    assert_eq!(Permission::from_mode("x"), Some(Permission::Execute));
    assert_eq!(Permission::from_mode("rw"), None);
    assert_eq!(Permission::from_mode(""), None);
}
