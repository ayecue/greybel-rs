use std::path::PathBuf;

use serde::Serialize;

use crate::bepinex::{BepInExInstallation, BepInExLauncher, BepInExMajor};
use crate::game::GameLayout;
use crate::hook::{HookConfig, HookInstaller, MessageHook};
use crate::platform::Platform;

/// Status of a game's message hook setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub folder: PathBuf,
    /// The game's program: `Grey Hack.app`, `Grey Hack.exe` or the `*.x86_64` program.
    pub executable: PathBuf,
    pub bepinex: Option<BepInExInstallation>,
    /// The assemblies in `BepInEx/plugins`.
    pub plugins: Vec<String>,
    /// The installed message hook build.
    pub hook: Option<String>,
    /// The configured port; the hook uses [`MessageHook::DEFAULT_PORT`] without one.
    pub port: Option<u16>,
    /// The Steam launch option macOS and Linux need to start the game with BepInEx.
    pub launch_option: Option<String>,
    /// A `libsteam_api.dylib` in the game's folder on macOS, which crashes the game under BepInEx because Steam's
    /// library is found twice.
    pub duplicate_steam_api: Option<PathBuf>,
    pub readiness: Readiness,
}

/// Whether the message hook can connect, and what is missing when not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Readiness {
    NoBepInEx,
    NoHook,
    /// `hook` is the build for the other BepInEx; BepInEx `major` needs the build `assembly_name`.
    WrongHook {
        hook: String,
        major: BepInExMajor,
        assembly_name: &'static str,
    },
    Ready {
        port: u16,
    },
}

impl GameStatus {
    #[must_use]
    pub fn read(game: &GameLayout, platform: Platform) -> Self {
        let bepinex = BepInExInstallation::detect(game);
        let installer = HookInstaller::new(game);
        let hook = installer.current_build();
        let port = HookConfig::new(game).port();
        let launch_option = (bepinex.is_some() && platform != Platform::Windows)
            .then(|| BepInExLauncher::new(game).steam_launch_option());
        let steam_api = game.folder().join("libsteam_api.dylib");
        let duplicate_steam_api =
            (bepinex.is_some() && platform == Platform::MacOS && steam_api.is_file())
                .then_some(steam_api);
        let readiness = match (&bepinex, &hook) {
            (None, _) => Readiness::NoBepInEx,
            (Some(_), None) => Readiness::NoHook,
            (Some(installation), Some(hook))
                if hook != MessageHook::assembly_name(installation.major) =>
            {
                Readiness::WrongHook {
                    hook: hook.clone(),
                    major: installation.major,
                    assembly_name: MessageHook::assembly_name(installation.major),
                }
            }
            (Some(_), Some(_)) => Readiness::Ready {
                port: port.unwrap_or(MessageHook::DEFAULT_PORT),
            },
        };

        Self {
            folder: game.folder().to_path_buf(),
            executable: game.executable().to_path_buf(),
            bepinex,
            plugins: installer.assemblies(),
            hook,
            port,
            launch_option,
            duplicate_steam_api,
            readiness,
        }
    }

    /// One sentence on whether the setup is done.
    #[must_use]
    pub fn readiness_text(&self) -> String {
        match &self.readiness {
            Readiness::NoBepInEx => "Not ready: BepInEx isn't installed.".to_owned(),
            Readiness::NoHook => "Not ready: the message hook isn't installed.".to_owned(),
            Readiness::WrongHook {
                hook,
                major,
                assembly_name,
            } => format!(
                "Not ready: {hook} is the message hook for another BepInEx version. BepInEx {} needs {assembly_name}.",
                major.number()
            ),
            Readiness::Ready { port } => {
                format!("Setup is done. The message hook connects on port {port}.")
            }
        }
    }
}
