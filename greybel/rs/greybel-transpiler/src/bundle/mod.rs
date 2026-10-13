mod bundler;
mod output;
mod template;

use bundler::Bundler;
pub use output::Output;
pub(crate) use template::{
    MULTILINE_STRING, REQUIRE, RUNTIME_NAMES, emit_multiline_string_function,
};

use crate::documents::{AsyncResourceProvider, ModuleGraph, ResourceProvider, Uri};
use crate::emit::{BeautifyEmitter, DefaultEmitter, UglifyEmitter};
use crate::error::{Result, TranspileError};
use crate::minify::Minification;
use crate::options::{BuildMode, TranspileOptions};

pub fn transpile_project(
    entry: &Uri,
    provider: &impl ResourceProvider,
    options: &TranspileOptions,
) -> Result<Vec<Output>> {
    let load_options = options.load_options();
    let graph = ModuleGraph::load(entry, &load_options, provider)?;

    transpile_graph(&graph, options)
}

pub async fn transpile_project_async(
    entry: &Uri,
    provider: &impl AsyncResourceProvider,
    options: &TranspileOptions,
) -> Result<Vec<Output>> {
    let load_options = options.load_options();
    let graph = ModuleGraph::load_async(entry, &load_options, provider).await?;

    transpile_graph(&graph, options)
}

pub fn transpile_graph(graph: &ModuleGraph, options: &TranspileOptions) -> Result<Vec<Output>> {
    let Some(main) = graph.entry() else {
        return Ok(Vec::new());
    };

    if let Some(cycle) = graph.cycle() {
        return Err(TranspileError::CircularDependency {
            from: cycle.from.clone(),
            to: cycle.to.clone(),
        });
    }

    match &options.mode {
        BuildMode::Default => Bundler::new(DefaultEmitter, graph, options)?.outputs(main),
        BuildMode::Uglify(uglify) => {
            let minification = Minification::analyze(&graph.dependency_order(main), options);
            let emitter = UglifyEmitter::new(*uglify, minification);

            Bundler::new(emitter, graph, options)?.outputs(main)
        }
        BuildMode::Beautify(beautify) => {
            let emitter = BeautifyEmitter::new(*beautify);

            Bundler::new(emitter, graph, options)?.outputs(main)
        }
    }
}
