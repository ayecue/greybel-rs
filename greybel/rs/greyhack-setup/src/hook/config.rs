use std::path::PathBuf;

use super::MessageHook;
use crate::error::{SetupError, SetupResult};
use crate::game::GameLayout;

/// The message hook's BepInEx configuration, `BepInEx/config/GreyHackMessageHook.cfg`, and its `General.Port`.
pub struct HookConfig<'a> {
    game: &'a GameLayout,
}

impl<'a> HookConfig<'a> {
    const GENERAL: &'static str = "[General]";

    #[must_use]
    pub fn new(game: &'a GameLayout) -> Self {
        Self { game }
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.game
            .config_folder()
            .join(format!("{}.cfg", MessageHook::GUID))
    }

    /// The configured port, `None` without a file or `Port` line.
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        let text = std::fs::read_to_string(self.path()).ok()?;

        text.lines().find_map(Self::port_of_line)
    }

    /// Sets `Port` to `port`, adding the line or the `[General]` section when missing.
    pub fn set_port(&self, port: u16) -> SetupResult<()> {
        let path = self.path();
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let folder = self.game.config_folder();

        std::fs::create_dir_all(&folder).map_err(SetupError::io(&folder))?;

        std::fs::write(&path, Self::with_port(&text, port)).map_err(SetupError::io(&path))
    }

    /// `config` with its first `Port = N` line set to `port`; without one, `Port` is added at the start of
    /// `[General]`, and without that section, the section is added at the end.
    #[must_use]
    pub fn with_port(config: &str, port: u16) -> String {
        let line = format!("Port = {port}");
        let lines: Vec<&str> = config.split_inclusive('\n').collect();

        if let Some(index) = lines
            .iter()
            .position(|line| Self::port_of_line(line.trim_end_matches(['\r', '\n'])).is_some())
        {
            let line_break = &lines[index][lines[index].trim_end_matches(['\r', '\n']).len()..];
            let mut lines: Vec<String> = lines.iter().map(|&line| line.to_owned()).collect();

            lines[index] = format!("{line}{line_break}");

            return lines.concat();
        }

        if let Some(index) = lines.iter().position(|line| {
            line.trim_end_matches(['\r', '\n']) == Self::GENERAL && line.ends_with('\n')
        }) {
            let mut lines: Vec<String> = lines.iter().map(|&line| line.to_owned()).collect();

            lines[index] = format!("{}\n\n{line}\n", Self::GENERAL);

            return lines.concat();
        }

        if config.is_empty() {
            format!("{}\n\n{line}\n", Self::GENERAL)
        } else {
            format!("{}\n\n{}\n\n{line}\n", config.trim_end(), Self::GENERAL)
        }
    }

    /// The port of a `Port = N` line.
    fn port_of_line(line: &str) -> Option<u16> {
        let value = line.strip_prefix("Port")?.trim_start_matches([' ', '\t']);
        let value = value.strip_prefix('=')?.trim_matches([' ', '\t', '\r']);

        (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| value.parse().ok())
            .flatten()
    }
}
