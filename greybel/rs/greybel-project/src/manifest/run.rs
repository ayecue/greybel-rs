use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::VariableSettings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RunSettings {
    /// Seed text for simulated-world generation; identical seeds produce identical worlds.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.seed"))]
    pub seed: String,
    /// Values for the program's `params`.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.defaultArgs", "x-vscode-format" = "spaceList"))]
    pub params: Vec<String>,
    /// Binary name used when building into the user's home folder so the program can launch itself. The in-game
    /// binary exists only for the run's duration.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.programName"))]
    pub program_name: String,
    /// Simulated-program memory limit in MiB before interruption, matching Grey Hack's out-of-memory behavior.
    /// Call nesting is unlimited.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.memoryLimit"))]
    pub memory_limit: u32,
    /// Account a simulated program runs as on the player's computer: the player's user, or `root` like after
    /// `sudo -s` in the game.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.user"))]
    pub user: RunUser,
    /// Run-time values for `#envar`.
    #[schemars(extend(
        "x-vscode-setting" = {
            "files": "greybel.interpreter.environmentFile",
            "values": "greybel.interpreter.environmentVariables"
        }
    ))]
    pub variables: VariableSettings,
    /// Splits long terminal lines as the greyhack-customizable-output mod does, the way the game did before
    /// 0.9.5683; absent splits them as the game does now.
    #[schemars(extend(
        "x-vscode-setting" = {
            "textPerLine": "greybel.interpreter.lineLimits.textPerLine",
            "tagsPerLine": "greybel.interpreter.lineLimits.tagsPerLine"
        }
    ))]
    pub line_limits: Option<LineLimits>,
}

/// Account a simulated program runs as.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RunUser {
    /// The player's user, with its home folder in `/home`.
    #[default]
    User,
    /// `root`, with its home folder `/root`.
    Root,
}

/// A line whose visible text is longer than `textPerLine` characters, or whose tags take more than `tagsPerLine`
/// characters, is cut into pieces of `textPerLine` characters. Characters count as UTF-16 code units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct LineLimits {
    #[schemars(range(min = 1))]
    pub text_per_line: u32,
    /// Absent or `null` for no limit.
    pub tags_per_line: Option<u32>,
}

impl Default for LineLimits {
    fn default() -> Self {
        Self {
            text_per_line: 256,
            tags_per_line: None,
        }
    }
}

impl RunSettings {
    pub const DEFAULT_MEMORY_LIMIT: u32 = 1024;
    pub const DEFAULT_PROGRAM_NAME: &str = "myprogram";
    pub const DEFAULT_SEED: &str = "test";
}

impl Default for RunSettings {
    fn default() -> Self {
        Self {
            seed: Self::DEFAULT_SEED.to_owned(),
            params: Vec::new(),
            program_name: Self::DEFAULT_PROGRAM_NAME.to_owned(),
            memory_limit: Self::DEFAULT_MEMORY_LIMIT,
            user: RunUser::User,
            variables: VariableSettings::default(),
            line_limits: None,
        }
    }
}
