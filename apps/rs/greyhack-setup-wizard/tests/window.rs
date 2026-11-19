use std::time::{Duration, Instant};

use eframe::egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};

use crate::support::FakeGame;

#[test]
fn should_offer_bepinex_5_and_the_hook_for_a_game_without_them() {
    let game = FakeGame::new();
    let mut window = game.window();

    window.run();
    window.get_by_label("Not ready: BepInEx isn't installed.");
    window.get_by_label("Apply…").click();
    window.run();

    window.get_by_label_contains("Download BepInEx 5.4.23.6 from https://github.com/BepInEx/");
    window.get_by_label_contains("Download the message hook of greyhack-message-hook/v");
    window.get_by_label("Cancel").click();
    window.run();

    assert!(window.query_by_label("Apply these changes?").is_none());
}

#[test]
fn should_have_nothing_to_change_once_set_up() {
    let game = FakeGame::new();

    game.write("BepInEx/core/BepInEx.dll")
        .write("BepInEx/plugins/GreyHackMessageHook5.dll");

    let mut window = game.window();

    window.run();
    window.get_by_label("Setup is done. The message hook connects on port 7777.");
    window.get_by_label("Nothing to change.");

    assert!(window.get_by_label("Apply…").accesskit_node().is_disabled());
}

#[test]
fn should_remove_the_hook_after_the_review() {
    let game = FakeGame::new();

    game.write("BepInEx/core/BepInEx.dll")
        .write("BepInEx/plugins/GreyHackMessageHook5.dll");

    let mut window = game.window();

    window.run();
    window.get_by_label("Remove the message hook").click();
    window.run();
    window.get_by_label("Apply…").click();
    window.run();
    window.get_by_role_and_label(Role::Label, "Remove the message hook");
    window.get_by_label("Apply").click();

    let deadline = Instant::now() + Duration::from_secs(10);

    while window.query_by_label("Setup complete.").is_none() {
        assert!(Instant::now() < deadline, "the setup finishes");
        std::thread::sleep(Duration::from_millis(20));
        window.step();
    }

    window.get_by_label("Not ready: the message hook isn't installed.");

    assert!(
        !game
            .path()
            .join("BepInEx/plugins/GreyHackMessageHook5.dll")
            .exists()
    );
}

#[test]
fn should_reject_a_port_outside_1_to_65535() {
    let game = FakeGame::new();

    game.write("BepInEx/core/BepInEx.dll")
        .write("BepInEx/plugins/GreyHackMessageHook5.dll");

    let mut window = game.window();

    window.run();

    let port = window.get_by_role_and_label(Role::TextInput, "Port");

    port.focus();
    port.type_text("0");
    window.run();

    window.get_by_label("Enter a port: a whole number from 1 to 65535.");
}

#[test]
fn should_show_the_launch_option_and_the_duplicate_steam_library() {
    let game = FakeGame::new();

    game.write("BepInEx/core/BepInEx.dll")
        .write("libsteam_api.dylib");

    let mut window = game.window();

    window.run();
    window.get_by_label_contains("run_bepinex.sh\" %command%");
    window.get_by_label("Remove the duplicate libsteam_api.dylib");
    window.get_by_label_contains("libsteam_api.dylib crashes Grey Hack under BepInEx");
}
