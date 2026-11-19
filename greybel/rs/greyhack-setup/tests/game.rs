use std::path::PathBuf;

use greyhack_setup::SetupError;
use greyhack_setup::game::{GameLayout, SteamLibraryCatalog};

use crate::support::TemporaryFolder;

#[test]
fn should_read_the_library_folders_steam_lists() {
    let library_folders = include_str!("fixtures/libraryfolders.vdf");

    let libraries = SteamLibraryCatalog::library_paths(library_folders);

    assert_eq!(
        libraries,
        [
            PathBuf::from(r"C:\Program Files (x86)\Steam"),
            PathBuf::from(r"D:\SteamLibrary")
        ]
    );
}

#[test]
fn should_find_the_game_in_a_library_with_its_manifest() {
    let steam = TemporaryFolder::new();
    let library = TemporaryFolder::new();

    steam.write(
        "steamapps/libraryfolders.vdf",
        &format!(
            "\"libraryfolders\" {{ \"1\" {{ \"path\" \"{}\" }} }}",
            library.path().display()
        ),
    );
    library.write(
        &format!("steamapps/appmanifest_{}.acf", SteamLibraryCatalog::APP_ID),
        "",
    );
    std::fs::create_dir_all(library.join("steamapps/common/Grey Hack")).expect("the game");

    let game_folder = SteamLibraryCatalog::new(vec![steam.path().to_path_buf()]).find_game();

    assert_eq!(
        game_folder,
        Some(library.join("steamapps/common/Grey Hack"))
    );
}

#[test]
fn should_find_no_game_without_its_manifest() {
    let steam = TemporaryFolder::new();

    std::fs::create_dir_all(steam.join("steamapps/common/Grey Hack")).expect("the game");

    let game_folder = SteamLibraryCatalog::new(vec![steam.path().to_path_buf()]).find_game();

    assert_eq!(game_folder, None);
}

#[test]
fn should_read_the_macos_games_data_folder_in_its_app() {
    let folder = TemporaryFolder::new();

    let game = folder.game();

    assert_eq!(
        game.managed_folder(),
        folder.join("Grey Hack.app/Contents/Resources/Data/Managed")
    );
}

#[test]
fn should_read_the_windows_games_data_folder_next_to_it() {
    let folder = TemporaryFolder::new();

    folder.write("Grey Hack.exe", "");

    let game = GameLayout::from_folder(folder.path()).expect("the game");

    assert_eq!(
        (game.executable(), game.data_folder()),
        (
            folder.join("Grey Hack.exe").as_path(),
            folder.join("Grey Hack_Data").as_path()
        )
    );
}

#[test]
fn should_read_the_linux_games_data_folder_next_to_it() {
    let folder = TemporaryFolder::new();

    folder.write("Grey Hack.x86_64", "");

    let game = GameLayout::from_folder(folder.path()).expect("the game");

    assert_eq!(game.data_folder(), folder.join("Grey Hack_Data"));
}

#[test]
fn should_reject_a_folder_without_the_game() {
    let folder = TemporaryFolder::new();

    let error = GameLayout::from_folder(folder.path()).expect_err("no game");

    assert!(matches!(error, SetupError::GameMissingInFolder(_)));
    assert!(error.to_string().starts_with("Grey Hack wasn't found in "));
}
