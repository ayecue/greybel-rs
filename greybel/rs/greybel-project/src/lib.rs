//! Reads a GreyScript project's `greybel.json`: defaulted settings ([`Manifest`]), manifest discovery
//! ([`Project::discover`]), and JSON Schema ([`Manifest::schema`]). The `builder` and `runtime` features construct
//! its builds and runs. All front-ends use this crate, giving the CLI, MCP server, language server, and greybel-vs
//! consistent behavior.

#[cfg(feature = "builder")]
mod builder;
mod manifest;
mod profile;
mod project;
#[cfg(feature = "runtime")]
mod runtime;
mod schema;

pub use manifest::{
    BuildMode, BuildSettings, DeploySettings, Format, Indentation, InstallerSettings, Manifest,
    RunSettings, VariableSettings,
};
pub use profile::{Profile, ProfileError};
pub use project::{Project, ProjectError, insert_assignments};
#[cfg(feature = "runtime")]
pub use runtime::Place;
