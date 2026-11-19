use greyhack_setup::hook::MessageHook;
use greyhack_setup::{
    BepInExSource, Downloader, GameStatus, HookSource, Platform, Readiness, Setup,
};

use crate::support::TemporaryFolder;

#[test]
fn should_be_ready_once_bepinex_the_hook_and_the_port_are_set_up() {
    let folder = TemporaryFolder::new();
    let cache = TemporaryFolder::new();
    let downloader = Downloader::new(cache.path().to_path_buf());
    let setup = Setup::new(folder.game(), Platform::MacOS, &downloader);
    let archive = folder.archive(
        "stable.zip",
        &[("run_bepinex.sh", ""), ("BepInEx/core/BepInEx.dll", "")],
    );
    let hook = folder.write("build/GreyHackMessageHook5.dll", "");
    let mut messages = Vec::new();
    let mut progress = |message: String| messages.push(message);

    assert_eq!(setup.status().readiness, Readiness::NoBepInEx);

    setup
        .install_bepinex(&BepInExSource::Archive(archive), &mut progress)
        .expect("BepInEx");

    assert_eq!(setup.status().readiness, Readiness::NoHook);

    setup
        .install_hook(None, &HookSource::Assembly(hook), &mut progress)
        .expect("the hook");
    setup.configure_port(7797, &mut progress).expect("the port");

    let status = setup.status();

    assert_eq!(status.readiness, Readiness::Ready { port: 7797 });
    assert_eq!(status.hook.as_deref(), Some("GreyHackMessageHook5"));
    assert_eq!(
        status.launch_option,
        Some(format!(
            "\"{}\" %command%",
            folder.join("run_bepinex.sh").display()
        ))
    );
    assert_eq!(messages.len(), 4);
}

#[test]
fn should_name_the_hook_build_bepinex_needs() {
    let folder = TemporaryFolder::new();
    let game = folder.game();

    folder.write("BepInEx/core/BepInEx.Unity.Mono.dll", "");
    folder.write("BepInEx/plugins/GreyHackMessageHook5.dll", "");

    let status = GameStatus::read(&game, Platform::Windows);

    assert_eq!(
        status.readiness_text(),
        "Not ready: GreyHackMessageHook5 is the message hook for another BepInEx version. BepInEx 6 needs GreyHackMessageHook."
    );
    assert_eq!(status.launch_option, None);
}

#[test]
fn should_find_the_duplicate_steam_library_that_crashes_bepinex_on_macos() {
    let folder = TemporaryFolder::new();
    let cache = TemporaryFolder::new();
    let downloader = Downloader::new(cache.path().to_path_buf());

    folder.write("BepInEx/core/BepInEx.dll", "");
    folder.write("libsteam_api.dylib", "");

    let setup = Setup::new(folder.game(), Platform::MacOS, &downloader);

    assert_eq!(
        setup.status().duplicate_steam_api,
        Some(folder.join("libsteam_api.dylib"))
    );

    setup
        .remove_duplicate_steam_api(&mut |_| {})
        .expect("removed");

    assert_eq!(setup.status().duplicate_steam_api, None);
}

#[test]
fn should_reject_a_port_outside_1_to_65535() {
    let folder = TemporaryFolder::new();
    let cache = TemporaryFolder::new();
    let downloader = Downloader::new(cache.path().to_path_buf());
    let setup = Setup::new(folder.game(), Platform::Linux, &downloader);

    let error = setup
        .configure_port(70_000, &mut |_| {})
        .expect_err("too large");

    assert_eq!(
        error.to_string(),
        "70000 isn't a valid port. Use a number from 1 to 65535."
    );
    assert_eq!(setup.status().port, None);
    assert_eq!(MessageHook::DEFAULT_PORT, 7777);
}

#[test]
fn should_describe_the_status_as_json() {
    let folder = TemporaryFolder::new();
    let game = folder.game();

    folder.write("BepInEx/core/BepInEx.dll", "");

    let json = serde_json::to_value(GameStatus::read(&game, Platform::Windows)).expect("JSON");

    assert_eq!(json["bepinex"]["major"], 5);
    assert_eq!(json["readiness"]["kind"], "noHook");
    assert_eq!(json["duplicateSteamApi"], serde_json::Value::Null);
}
