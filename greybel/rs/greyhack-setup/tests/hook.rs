use greyhack_setup::bepinex::BepInExMajor;
use greyhack_setup::hook::{HookConfig, HookInstaller, HookRelease, MessageHook};

use crate::support::TemporaryFolder;

#[test]
fn should_replace_the_build_of_the_other_major() {
    let folder = TemporaryFolder::new();
    let game = folder.game();

    folder.write("BepInEx/plugins/GreyHackMessageHook5.dll", "");
    folder.write("BepInEx/plugins/GreyHackMessageHook5.pdb", "");

    let build = folder.write("build/GreyHackMessageHook.dll", "beta build");

    folder.write("build/GreyHackMessageHook.pdb", "symbols");

    HookInstaller::new(&game)
        .install(BepInExMajor::Beta, &build)
        .expect("installed");

    assert_eq!(
        HookInstaller::new(&game).assemblies(),
        ["GreyHackMessageHook"]
    );
    assert!(
        game.plugins_folder()
            .join("GreyHackMessageHook.pdb")
            .exists()
    );
    assert!(
        !game
            .plugins_folder()
            .join("GreyHackMessageHook5.pdb")
            .exists()
    );
}

#[test]
fn should_install_into_a_game_without_a_plugins_folder() {
    let folder = TemporaryFolder::new();
    let game = folder.game();
    let build = folder.write("build/GreyHackMessageHook5.dll", "stable build");

    HookInstaller::new(&game)
        .install(BepInExMajor::Stable, &build)
        .expect("installed");

    assert_eq!(
        HookInstaller::new(&game).assemblies(),
        ["GreyHackMessageHook5"]
    );
}

#[test]
fn should_reject_the_build_of_another_major() {
    let folder = TemporaryFolder::new();
    let game = folder.game();
    let build = folder.write("build/GreyHackMessageHook5.dll", "stable build");

    let error = HookInstaller::new(&game)
        .install(BepInExMajor::Beta, &build)
        .expect_err("the stable build");

    assert!(
        error
            .to_string()
            .ends_with("is the wrong plugin file. BepInEx 6 needs GreyHackMessageHook.dll.")
    );
}

#[test]
fn should_write_a_new_config_with_the_port() {
    let folder = TemporaryFolder::new();
    let game = folder.game();
    let config = HookConfig::new(&game);

    config.set_port(7797).expect("set");

    assert_eq!(
        std::fs::read_to_string(config.path()).expect("the config"),
        "[General]\n\nPort = 7797\n"
    );
    assert_eq!(config.port(), Some(7797));
}

#[test]
fn should_change_only_the_port_of_the_config_the_plugin_wrote() {
    let config = "## Settings file\n\n[General]\n\n## Port\n# Setting type: Int32\nPort = 7777\n\nFile Extensions = src,gs,ms\n";

    let config_with_port = HookConfig::with_port(config, 7797);

    assert_eq!(
        config_with_port,
        config.replace("Port = 7777", "Port = 7797")
    );
}

#[test]
fn should_add_the_port_to_the_general_section_without_it() {
    let config_with_port = HookConfig::with_port("[General]\nFile Extensions = src\n", 7797);

    assert_eq!(
        config_with_port,
        "[General]\n\nPort = 7797\nFile Extensions = src\n"
    );
}

#[test]
fn should_add_the_general_section_to_a_config_without_it() {
    let config_with_port = HookConfig::with_port("[Other]\nValue = 1\n", 7797);

    assert_eq!(
        config_with_port,
        "[Other]\nValue = 1\n\n[General]\n\nPort = 7797\n"
    );
}

#[test]
fn should_pick_the_build_of_the_major_with_its_sha256() {
    let release = HookRelease::parse(include_str!("fixtures/release.json")).expect("a release");

    let asset = release
        .asset(BepInExMajor::Stable)
        .expect("the stable build");

    assert_eq!(
        (asset.name.as_str(), asset.sha256.as_deref()),
        ("GreyHackMessageHook5.dll", Some("a".repeat(64).as_str()))
    );
}

#[test]
fn should_reject_a_build_github_records_no_sha256_for() {
    let release = HookRelease::parse(include_str!("fixtures/release.json")).expect("a release");

    let error = release.asset(BepInExMajor::Beta).expect_err("no digest");

    assert_eq!(
        error.to_string(),
        "Release v0.1.0 has no checksum for GreyHackMessageHook.dll, so it can't be verified."
    );
}

#[test]
fn should_install_the_hook_release_of_its_own_commit() {
    let project =
        include_str!("../../../dotnet/greyhack-message-hook/src/GreyHackMessageHook.csproj");

    assert!(project.contains(&format!("<Version>{}</Version>", MessageHook::VERSION)));
    assert_eq!(
        MessageHook::release_tag(),
        format!("greyhack-message-hook/v{}", MessageHook::VERSION)
    );
}
