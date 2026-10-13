use greybel_project::BuildMode;
use schemars::JsonSchema;
use serde::Deserialize;

/// Transpiled-code output mode.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Default,
    Uglify,
    Beautify,
}

impl From<Mode> for BuildMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Default => Self::Default,
            Mode::Uglify => Self::Uglify,
            Mode::Beautify => Self::Beautify,
        }
    }
}
