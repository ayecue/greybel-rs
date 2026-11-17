//! Reads a GreyScript project's `greybel.json`: defaulted settings ([`Manifest`]), manifest discovery
//! ([`Project::discover`]), and JSON Schema ([`Manifest::schema`]). The `builder` and `runtime` features construct
//! its builds and runs. All front-ends use this crate, giving the CLI, MCP server, language server, and greybel-vs
//! consistent behavior.

mod manifest;
mod project;

pub use manifest::{
    BuildMode, BuildSettings, DeploySettings, DiagnosticLevel, DiagnosticSettings, Format,
    Indentation, InstallerSettings, Manifest, MultilineStringStyle, Profile, ProfileError,
    RunSettings, SyntaxSettings, VariableFile, VariableSettings,
};
#[cfg(feature = "runtime")]
pub use project::Place;
pub use project::{Project, ProjectError};
