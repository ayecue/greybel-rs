use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(into = "i32")]
pub enum BuildState {
    #[default]
    Initial,
    Allocation,
    PreScript,
    EntityCreation,
    PostScript,
    Cleanup,
    Complete,
    Failure,
    Unknown,
    Other(i32),
}

impl From<i32> for BuildState {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Initial,
            1 => Self::Allocation,
            2 => Self::PreScript,
            3 => Self::EntityCreation,
            4 => Self::PostScript,
            5 => Self::Cleanup,
            10 => Self::Complete,
            20 => Self::Failure,
            30 => Self::Unknown,
            other => Self::Other(other),
        }
    }
}

impl From<BuildState> for i32 {
    fn from(value: BuildState) -> Self {
        match value {
            BuildState::Initial => 0,
            BuildState::Allocation => 1,
            BuildState::PreScript => 2,
            BuildState::EntityCreation => 3,
            BuildState::PostScript => 4,
            BuildState::Cleanup => 5,
            BuildState::Complete => 10,
            BuildState::Failure => 20,
            BuildState::Unknown => 30,
            BuildState::Other(other) => other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(into = "i32")]
pub enum ScriptType {
    #[default]
    Pre,
    Post,
    Other(i32),
}

impl From<i32> for ScriptType {
    fn from(value: i32) -> Self {
        match value {
            10 => Self::Pre,
            20 => Self::Post,
            other => Self::Other(other),
        }
    }
}

impl From<ScriptType> for i32 {
    fn from(value: ScriptType) -> Self {
        match value {
            ScriptType::Pre => 10,
            ScriptType::Post => 20,
            ScriptType::Other(other) => other,
        }
    }
}
