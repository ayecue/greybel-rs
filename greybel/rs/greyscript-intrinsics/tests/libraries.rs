use greybel_test_support::future::block_on;
use greyhack_behaviour::{FIRST_VERSION, FileData, LibraryKind};
use greyhack_world::{Library, PlayerOptions, World};

use crate::support::add_library;
use crate::support::scripts::run_on_host;

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
