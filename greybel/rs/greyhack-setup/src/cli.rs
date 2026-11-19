//! The `setup` command line of greyhack-setup and `greybel setup`. `--json` writes one JSON object per line instead
//! of text: `progress` events while a step runs, then a `status` or `done` event, or an `error` event.

use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::bepinex::{BepInExMajor, BepInExRelease};
use crate::game::GameLocator;
use crate::hook::{HookRelease, MessageHook};
use crate::{
    BepInExSource, Downloader, GameStatus, HookSource, Platform, Setup, SetupError, SetupResult,
};

#[derive(Debug, Args)]
pub struct SetupCli {
    /// Grey Hack's folder (default: GREY_HACK_PATH, then your Steam libraries)
    #[arg(long, global = true, value_name = "FOLDER")]
    pub game: Option<PathBuf>,
    /// Folder for downloaded files
    #[arg(long, global = true, value_name = "FOLDER")]
    pub cache: Option<PathBuf>,
    /// Download without confirmation
    #[arg(long, short, global = true)]
    pub yes: bool,
    /// Write JSON lines instead of text
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: SetupCommand,
}

#[derive(Debug, Subcommand)]
pub enum SetupCommand {
    /// Show where Grey Hack is installed and what is set up
    #[command(alias = "locate")]
    Status,
    /// Install or remove BepInEx, the mod loader
    #[command(subcommand)]
    Bepinex(BepInExCommand),
    /// Install or remove the message hook plugin
    #[command(subcommand)]
    Hook(HookCommand),
    /// Set the port the message hook listens on
    Configure {
        /// The port (the hook's default is 7777)
        #[arg(long)]
        port: u32,
    },
    /// Remove the libsteam_api.dylib in the game's folder that crashes Grey Hack under BepInEx on macOS
    RemoveDuplicateSteamApi,
}

#[derive(Debug, Subcommand)]
pub enum BepInExCommand {
    /// Install BepInEx, replacing any installed version; plugins and settings stay
    Install {
        /// BepInEx version: 5 (stable, recommended) or 6 (beta)
        #[arg(long)]
        bepinex: BepInExMajor,
        /// Install from this BepInEx zip file instead of downloading
        #[arg(long, value_name = "FILE")]
        zip: Option<PathBuf>,
    },
    /// Remove BepInEx; plugins and settings stay
    Remove,
}

#[derive(Debug, Subcommand)]
pub enum HookCommand {
    /// Install the message hook for the installed BepInEx
    Install {
        /// Install this plugin file (GreyHackMessageHook5.dll for BepInEx 5, GreyHackMessageHook.dll for BepInEx 6)
        #[arg(long, value_name = "FILE", conflicts_with = "release")]
        assembly: Option<PathBuf>,
        /// Download the hook of this release tag, or "latest" (default: the hook of this build's commit)
        #[arg(long, value_name = "TAG")]
        release: Option<String>,
        /// Install the build for this BepInEx version (default: the installed one)
        #[arg(long)]
        bepinex: Option<BepInExMajor>,
        /// GitHub repository to download releases from
        #[arg(long, value_name = "OWNER/REPO", default_value = MessageHook::DEFAULT_REPOSITORY)]
        repository: String,
    },
    /// Remove the message hook
    Remove,
}

impl SetupCli {
    /// Runs the command, writing to stdout; errors are written as a line too.
    #[must_use]
    pub fn run(self) -> ExitCode {
        let output = Output { json: self.json };

        match self.execute(&output) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                output.error(&error.to_string());

                ExitCode::FAILURE
            }
        }
    }

    fn execute(&self, output: &Output) -> SetupResult<()> {
        let platform = Platform::current()?;
        let game = GameLocator::new(platform).require(self.game.as_deref())?;
        let downloader = Downloader::new(
            self.cache
                .clone()
                .unwrap_or_else(|| Downloader::default_cache_folder(platform)),
        );
        let setup = Setup::new(game, platform, &downloader);
        let mut progress = |message: String| output.progress(&message);

        match &self.command {
            SetupCommand::Status => {
                output.status(&setup.status(), platform);

                return Ok(());
            }
            SetupCommand::Bepinex(BepInExCommand::Install { bepinex, zip }) => {
                let source = match zip {
                    Some(zip) => BepInExSource::Archive(zip.clone()),
                    None => {
                        let release = setup.bepinex_release(*bepinex);

                        self.consent(
                            output,
                            &format!("Download BepInEx {} from {}?", release.version, release.url),
                            &format!(
                                "Nothing was downloaded. To install it yourself, download {} and pass it with --zip.",
                                release.url
                            ),
                        )?;

                        BepInExSource::Download(*bepinex)
                    }
                };

                setup.install_bepinex(&source, &mut progress)?;
            }
            SetupCommand::Bepinex(BepInExCommand::Remove) => setup.remove_bepinex(&mut progress)?,
            SetupCommand::Hook(HookCommand::Install {
                assembly,
                release,
                bepinex,
                repository,
            }) => {
                let source = match assembly {
                    Some(assembly) => HookSource::Assembly(assembly.clone()),
                    None => {
                        let major = match bepinex {
                            Some(major) => *major,
                            None => setup.current_bepinex_major()?,
                        };
                        let tag = match release.as_deref() {
                            Some("latest") => None,
                            Some(tag) => Some(tag.to_owned()),
                            None => Some(MessageHook::release_tag()),
                        };
                        let release = HookRelease::find(&downloader, repository, tag.as_deref())?;
                        let asset = release.asset(major)?;

                        self.consent(
                            output,
                            &format!("Download {} ({}) from {}?", asset.name, release.tag, asset.url),
                            &format!(
                                "Nothing was downloaded. To install it yourself, download {} and pass it with --assembly.",
                                asset.url
                            ),
                        )?;

                        HookSource::Release(release)
                    }
                };

                setup.install_hook(*bepinex, &source, &mut progress)?;
            }
            SetupCommand::Hook(HookCommand::Remove) => setup.remove_hook(&mut progress)?,
            SetupCommand::Configure { port } => setup.configure_port(*port, &mut progress)?,
            SetupCommand::RemoveDuplicateSteamApi => {
                setup.remove_duplicate_steam_api(&mut progress)?
            }
        }

        output.finish(&setup.status(), platform);

        Ok(())
    }

    /// Requests confirmation before a download unless `--yes` is given; without a terminal, nothing is downloaded.
    fn consent(&self, output: &Output, question: &str, refusal_text: &str) -> SetupResult<()> {
        if self.yes {
            return Ok(());
        }

        let stdin = std::io::stdin();

        if !output.json && stdin.is_terminal() {
            print!("{question} [y/N] ");

            let _ = std::io::stdout().flush();

            let mut response = String::new();
            let _ = stdin.lock().read_line(&mut response);

            if response.trim().eq_ignore_ascii_case("y") {
                return Ok(());
            }
        }

        Err(SetupError::Refusal(format!(
            "{refusal_text} Pass --yes to download without asking."
        )))
    }
}

/// Writes the command's results as text or JSON lines.
struct Output {
    json: bool,
}

impl Output {
    fn progress(&self, message: &str) {
        self.write(&json!({ "type": "progress", "message": message }), message);
    }

    fn error(&self, message: &str) {
        if self.json {
            println!("{}", json!({ "type": "error", "message": message }));
        } else {
            eprintln!("Error: {message}");
        }
    }

    fn status(&self, status: &GameStatus, platform: Platform) {
        self.write(
            &Self::status_event("status", status, platform),
            &Self::status_text(status),
        );
    }

    fn finish(&self, status: &GameStatus, platform: Platform) {
        if self.json {
            println!("{}", Self::status_event("done", status, platform));
        } else {
            println!("{}", status.readiness_text());
        }
    }

    fn write(&self, event: &Value, text: &str) {
        if self.json {
            println!("{event}");
        } else {
            println!("{text}");
        }
    }

    /// The status, with what a download would fetch: the BepInEx archives for this platform and the hook's tag.
    fn status_event(kind: &str, status: &GameStatus, platform: Platform) -> Value {
        let releases: Vec<Value> = BepInExMajor::ALL
            .into_iter()
            .map(|major| {
                let release = BepInExRelease::of(major, platform);

                json!({ "major": major, "version": release.version, "url": release.url })
            })
            .collect();

        json!({
            "type": kind,
            "status": status,
            "readinessText": status.readiness_text(),
            "bepinexReleases": releases,
            "hookRelease": MessageHook::release_tag(),
        })
    }

    fn status_text(status: &GameStatus) -> String {
        let absent = || "not installed".to_owned();
        let mut lines = vec![
            format!("Game:    {}", status.folder.display()),
            format!(
                "BepInEx: {}",
                status
                    .bepinex
                    .as_ref()
                    .map_or_else(absent, |installation| installation.description())
            ),
            format!("Hook:    {}", status.hook.clone().unwrap_or_else(absent)),
            format!(
                "Port:    {}",
                status.port.map_or_else(
                    || format!("{} (default)", MessageHook::DEFAULT_PORT),
                    |port| port.to_string()
                )
            ),
            format!(
                "Plugins: {}",
                if status.plugins.is_empty() {
                    "none".to_owned()
                } else {
                    status.plugins.join(", ")
                }
            ),
        ];

        if let Some(option) = &status.launch_option {
            lines.push(format!(
                "Start the game from Steam with the launch option: {option}"
            ));
        }

        if let Some(path) = &status.duplicate_steam_api {
            lines.push(format!(
                "{} crashes Grey Hack under BepInEx; remove it with remove-duplicate-steam-api.",
                path.display()
            ));
        }

        lines.push(status.readiness_text());

        lines.join("\n")
    }
}
