pub use greybel_documents as documents;
pub use greybel_documents::DocumentError;

pub mod bundle;
pub mod emit;
pub mod error;
pub mod minify;
pub mod names;
pub mod options;
pub mod transpile;

pub use bundle::{Output, transpile_graph, transpile_project, transpile_project_async};
pub use documents::{AsyncResourceProvider, ResourceProvider};
pub use emit::{BeautifyEmitter, DefaultEmitter, EmitFlags, EmitState, Emitter, UglifyEmitter};
pub use error::{Result, TranspileError};
pub use minify::Minification;
pub use options::{
    BeautifyOptions, BuildMode, Indentation, PathMapping, TranspileOptions, UglifyOptions,
};
pub use transpile::{emit, parse, transpile};
