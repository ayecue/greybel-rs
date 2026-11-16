use greybel_test_support::future::block_on;
use greyhack_behaviour::{FIRST_VERSION, FileData, LibraryKind};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_libraries::Library;

use crate::support::add_library;
use crate::support::scripts::run_on_host;
use crate::support::system_log::world_with_service;

#[test]
fn should_fail_when_including_malformed_library_json() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.handle.player();
    let mut data = FileData::library(0);

    data.content = "not json".to_owned();
    add_library(&mut world, player, "bad.so", data, None);

    let (output, error, _) = run_on_host("include_lib(\"/lib/bad.so\")", world, true);

    assert_eq!(output, "");
    assert!(error.is_some());
}

#[test]
fn should_fail_when_net_use_reads_malformed_library_json() {
    let (mut world, target) = world_with_service();
    let mut data = FileData::library(0);

    data.content = "not json".to_owned();
    add_library(
        &mut world,
        target.computer,
        &format!("lib{}.so", target.service),
        data,
        None,
    );

    let script = format!(
        "include_lib(\"/lib/metaxploit.so\").net_use(\"{}\", {})",
        target.address, target.port
    );
    let (output, error, _) = run_on_host(&script, world, true);

    assert_eq!(output, "");
    assert!(error.is_some());
}

#[test]
fn should_return_null_when_a_net_session_library_is_no_longer_a_library() {
    let (world, target) = world_with_service();
    let script = format!(
        "remote = get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\"); pc = remote.host_computer; mx = include_lib(\"/lib/metaxploit.so\"); session = mx.net_use(\"{}\", {}); pc.touch(\"/root\", \"notlib.txt\"); pc.File(\"/root/notlib.txt\").move(\"/lib\", \"lib{}.so\"); print session.dump_lib",
        target.address,
        target.port,
        target.user,
        target.password,
        target.address,
        target.port,
        target.service
    );
    let (output, error, _) = run_on_host(&script, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "null");
}

#[test]
fn should_reject_a_non_numeric_is_patched_argument() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, _) = run_on_host(
        "mx = include_lib(\"/lib/metaxploit.so\"); print mx.load(\"/lib/metaxploit.so\").is_patched(\"x\")",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(output, "null");
}

#[test]
fn should_include_a_library_by_its_content_rather_than_its_name() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.handle.player();

    add_library(
        &mut world,
        player,
        "renamed.so",
        FileData::library(0),
        Some(Library {
            kind: LibraryKind::Crypto,
            version: FIRST_VERSION.to_owned(),
            zones: Vec::new(),
        }),
    );
    add_library(
        &mut world,
        player,
        "libsmtp.so",
        FileData::library(0),
        Some(Library {
            kind: LibraryKind::Smtp,
            version: FIRST_VERSION.to_owned(),
            zones: Vec::new(),
        }),
    );
    add_library(
        &mut world,
        player,
        "libftp.so",
        FileData::library(0),
        Some(Library {
            kind: LibraryKind::Ftp,
            version: FIRST_VERSION.to_owned(),
            zones: Vec::new(),
        }),
    );
    add_library(&mut world, player, "empty.so", FileData::library(0), None);

    let code = r#"
print typeof(include_lib("/lib/renamed.so"))
print typeof(include_lib("/lib/libsmtp.so"))
print typeof(include_lib("/lib/libftp.so"))
print typeof(include_lib("/lib/empty.so"))
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "cryptoLib\nnull\nservice\nnull");
}
