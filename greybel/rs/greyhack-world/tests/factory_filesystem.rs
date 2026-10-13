use greyhack_behaviour::{FileData, FileKind, GUEST_USER, ROOT_USER};
use greyhack_world::PlayerOptions;
use greyhack_world_assets::{FileSystemLayout, Script};

use crate::support::factory_filesystem::{
    data, names, npc, npc_without_record_viewer, player, world,
};

#[tokio::test]
async fn should_lay_out_a_computer_as_the_games_base_file_system() {
    let world = world().await;
    let npc = npc(&world).await;
    let bin = FileSystemLayout::base()
        .folder("bin")
        .expect("the layout has /bin");

    let mut found = names(&world, npc.id, "/bin").await;

    found.sort();

    let mut expected: Vec<String> = bin.files.iter().map(|file| file.name.clone()).collect();

    expected.sort();
    assert_eq!(found, expected);
}

#[tokio::test]
async fn should_give_the_bin_commands_their_game_sources() {
    let world = world().await;
    let npc = npc(&world).await;

    let ls = data(&world, npc.id, "/bin/ls").await;

    assert_eq!(
        (
            ls.kind,
            ls.is_binary(),
            ls.is_executable(),
            ls.content.as_str()
        ),
        (
            FileKind::Program,
            true,
            true,
            Script::find("ls").expect("ls").source
        )
    );
}

#[tokio::test]
async fn should_separate_the_users_file_in_etc_from_the_passwd_command() {
    let world = world().await;
    let npc = npc(&world).await;

    let (users, command) = (
        data(&world, npc.id, "/etc/passwd").await,
        data(&world, npc.id, "/bin/passwd").await,
    );

    assert!(users.content.starts_with("root:"), "{}", users.content);
    assert_eq!(
        command.content,
        Script::find("passwd").expect("passwd").source
    );
}

#[tokio::test]
async fn should_add_the_first_install_files_only_to_the_players_computer() {
    let world = world().await;
    let player = player(&world).await;
    let npc = npc(&world).await;
    let files = ["/bin/aircrack", "/usr/bin/Map.exe", "/lib/crypto.so"];
    let mut present = Vec::new();

    for path in files {
        present.push((
            world.file(player.id, path, false).await.unwrap().is_some(),
            world.file(npc.id, path, false).await.unwrap().is_some(),
        ));
    }

    assert_eq!(present, [(true, false); 3]);
}

#[tokio::test]
async fn should_give_the_first_install_files_to_the_player() {
    let world = world().await;
    let player = player(&world).await;

    let owner = world
        .file(player.id, "/bin/aircrack", false)
        .await
        .unwrap()
        .map(|node| node.owner);

    assert_eq!(
        owner.as_deref(),
        Some(PlayerOptions::default().user.as_str())
    );
}

#[tokio::test]
async fn should_give_every_user_and_guest_a_home() {
    let world = world().await;
    let npc = npc(&world).await;

    let mut homes = names(&world, npc.id, "/home").await;

    homes.sort();

    let mut expected: Vec<String> = world
        .users(npc.id)
        .await
        .unwrap()
        .iter()
        .map(|user| user.name.clone())
        .filter(|name| name != ROOT_USER)
        .chain([GUEST_USER.to_owned()])
        .collect();

    expected.sort();
    assert_eq!(homes, expected);
}

#[tokio::test]
async fn should_give_root_a_home_with_the_default_folders() {
    let world = world().await;
    let npc = npc_without_record_viewer(&world).await;

    let root_home = names(&world, npc.id, "/root").await;

    assert_eq!(root_home, ["Desktop", "Downloads", "Config", ".Trash"]);
}

#[test]
fn should_size_binaries_from_their_names_as_the_game_does() {
    let files = [("ls", false), ("init.so", true)];

    let sizes = files.map(|(name, library)| FileData::binary_size(name, library));

    assert_eq!(sizes, [2_472_628, 542_541]);
}
