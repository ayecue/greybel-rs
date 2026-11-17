pub use greybel_documents::DocumentError;

pub mod bundle;
pub mod emit;
pub mod error;
pub mod minify;
pub mod names;
pub mod options;
pub mod transpiler;

pub use bundle::Output;
pub use emit::{BeautifyEmitter, DefaultEmitter, EmitFlags, EmitState, Emitter, UglifyEmitter};
pub use error::{Result, TranspileError};
pub use greybel_documents::{AsyncResourceProvider, ResourceProvider};
pub use minify::Minification;
pub use options::{
    BeautifyOptions, BuildMode, Indentation, MultilineStringStyle, PathMapping, TranspileOptions,
    UglifyOptions,
};
pub use transpiler::Transpiler;
