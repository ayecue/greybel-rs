use greyhack_setup::bepinex::{
    BepInExInstallation, BepInExInstaller, BepInExLauncher, BepInExMajor, BepInExRelease,
};
use greyhack_setup::{Platform, SetupError};

use crate::support::TemporaryFolder;

#[test]
fn should_replace_the_loader_and_core_but_not_the_plugins_and_config() {
    let folder = TemporaryFolder::new();
    let game = folder.game();

    folder.write("BepInEx/core/BepInEx.dll", "");
    folder.write("BepInEx/core/Old.dll", "");
    folder.write("libdoorstop.dylib", "old");
    folder.write("BepInEx/plugins/Mine.dll", "");
    folder.write("BepInEx/config/Mine.cfg", "");
    folder.write("BepInEx/LogOutput.log", "");

    let archive = folder.archive(
        "beta.zip",
        &[
            ("run_bepinex.sh", "run_bepinex.sh"),
            ("libdoorstop.dylib", "libdoorstop.dylib"),
            ("BepInEx/core/BepInEx.Unity.Mono.dll", ""),
            ("BepInEx/core/BepInEx.Core.dll", ""),
        ],
    );

    let installation = BepInExInstaller::new(&game)
        .install(&archive)
        .expect("installed");

    assert_eq!(installation.major, BepInExMajor::Beta);
    assert!(!game.bepinex_folder().join("core/Old.dll").exists());
    assert!(!game.bepinex_folder().join("LogOutput.log").exists());
    assert_eq!(
        std::fs::read_to_string(folder.join("libdoorstop.dylib")).expect("the loader"),
        "libdoorstop.dylib"
    );
    assert!(game.plugins_folder().join("Mine.dll").exists());
    assert!(game.config_folder().join("Mine.cfg").exists());
}

#[cfg(unix)]
#[test]
fn should_make_the_launcher_executable() {
    use std::os::unix::fs::PermissionsExt;

    let folder = TemporaryFolder::new();
    let game = folder.game();
    let archive = folder.archive(
        "stable.zip",
        &[("run_bepinex.sh", ""), ("BepInEx/core/BepInEx.dll", "")],
    );

    BepInExInstaller::new(&game)
        .install(&archive)
        .expect("installed");

    let mode = std::fs::metadata(folder.join("run_bepinex.sh"))
        .expect("the launcher")
        .permissions()
        .mode();

    assert_eq!(mode & 0o100, 0o100);
}

#[test]
fn should_run_the_beta_as_x64_on_macos() {
    let folder = TemporaryFolder::new();
    let game = folder.game();
    let archive = folder.archive(
        "beta.zip",
        &[
            (
                "run_bepinex.sh",
                "#!/bin/sh\narchpreference=\"arm64,x86_64\"\nexec \"$@\"\n",
            ),
            ("BepInEx/core/BepInEx.Unity.Mono.dll", ""),
        ],
    );

    BepInExInstaller::new(&game)
        .install(&archive)
        .expect("installed");

    assert_eq!(
        BepInExLauncher::new(&game).architectures().as_deref(),
        Some("x86_64")
    );
    assert_eq!(
        std::fs::read_to_string(folder.join("run_bepinex.sh")).expect("the launcher"),
        "#!/bin/sh\narchpreference=\"x86_64\"\nexec \"$@\"\n"
    );
}

#[test]
fn should_run_the_stable_as_the_launcher_prefers() {
    let folder = TemporaryFolder::new();
    let game = folder.game();
    let archive = folder.archive(
        "stable.zip",
        &[
            (
                "run_bepinex.sh",
                "#!/bin/sh\narchpreference=\"arm64,x86_64\"\n",
            ),
            ("BepInEx/core/BepInEx.dll", ""),
        ],
    );

    BepInExInstaller::new(&game)
        .install(&archive)
        .expect("installed");

    assert_eq!(
        BepInExLauncher::new(&game).architectures().as_deref(),
        Some("arm64,x86_64")
    );
}

#[test]
fn should_reject_an_archive_without_bepinexs_core() {
    let folder = TemporaryFolder::new();
    let game = folder.game();
    let archive = folder.archive("other.zip", &[("readme.txt", "")]);

    let error = BepInExInstaller::new(&game)
        .install(&archive)
        .expect_err("not BepInEx");

    assert!(
        error
            .to_string()
            .ends_with("isn't a BepInEx zip file (it has no BepInEx/core folder).")
    );
}

#[test]
fn should_remove_the_loader_and_core_but_not_the_plugins_and_config() {
    let folder = TemporaryFolder::new();
    let game = folder.game();

    folder.write("BepInEx/core/BepInEx.dll", "");
    folder.write("winhttp.dll", "");
    folder.write("BepInEx/plugins/Mine.dll", "");
    folder.write("BepInEx/config/Mine.cfg", "");

    BepInExInstaller::new(&game).remove().expect("removed");

    assert!(BepInExInstallation::detect(&game).is_none());
    assert!(!folder.join("winhttp.dll").exists());
    assert!(game.plugins_folder().join("Mine.dll").exists());
    assert!(game.config_folder().join("Mine.cfg").exists());
}

#[test]
fn should_detect_no_bepinex_in_a_game_without() {
    let folder = TemporaryFolder::new();

    assert_eq!(BepInExInstallation::detect(&folder.game()), None);
}

#[test]
fn should_detect_bepinex_without_a_readable_version() {
    let folder = TemporaryFolder::new();
    let game = folder.game();

    folder.write("BepInEx/core/BepInEx.dll", "not an assembly");

    let installation = BepInExInstallation::detect(&game).expect("BepInEx");

    assert_eq!(installation.description(), "BepInEx 5");
}

#[test]
fn should_escape_the_build_metadata_of_the_beta_address() {
    let release = BepInExRelease::of(BepInExMajor::Beta, Platform::Linux);

    assert_eq!(
        release.url,
        "https://builds.bepinex.dev/projects/bepinex_be/791/BepInEx-Unity.Mono-linux-x64-6.0.0-be.791%2Bb782da4.zip"
    );
}

#[test]
fn should_reject_a_bepinex_version_other_than_5_or_6() {
    assert_eq!("5".parse::<BepInExMajor>(), Ok(BepInExMajor::Stable));
    assert_eq!("6".parse::<BepInExMajor>(), Ok(BepInExMajor::Beta));
    assert!("7".parse::<BepInExMajor>().is_err());
}

#[test]
fn should_report_a_launcher_without_an_architecture_preference() {
    let folder = TemporaryFolder::new();
    let game = folder.game();

    folder.write("run_bepinex.sh", "#!/bin/sh\n");

    let error = BepInExLauncher::new(&game)
        .prefer_architectures("x86_64")
        .expect_err("no archpreference");

    assert!(matches!(
        error,
        SetupError::LauncherPreferenceMissing { .. }
    ));
}
