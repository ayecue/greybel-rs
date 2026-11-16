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
    /// Run-time values for `#envar`.
    #[schemars(extend(
        "x-vscode-setting" = {
            "files": "greybel.interpreter.environmentFile",
            "values": "greybel.interpreter.environmentVariables"
        }
    ))]
    pub variables: VariableSettings,
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
            variables: VariableSettings::default(),
        }
    }
}
