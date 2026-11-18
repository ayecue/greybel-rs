mod bundler;
mod output;
mod template;

use bundler::Bundler;
use greybel_documents::{AsyncResourceProvider, ModuleGraph, ResourceProvider, Uri};
pub use output::Output;
pub(crate) use template::{
    REQUIRE, RUNTIME_NAMES, emit_multiline_string_function, multiline_string_helper,
};

use crate::emit::{BeautifyEmitter, DefaultEmitter, UglifyEmitter};
use crate::error::{Result, TranspileError};
use crate::minify::Minification;
use crate::options::BuildMode;
use crate::transpiler::Transpiler;

impl Transpiler<'_> {
    pub fn transpile_project(
        &self,
        entry: &Uri,
        provider: &impl ResourceProvider,
    ) -> Result<Vec<Output>> {
        let load_options = self.options().load_options();
        let graph = ModuleGraph::load(entry, &load_options, provider)?;

        self.transpile_graph(&graph)
    }

    pub async fn transpile_project_async(
        &self,
        entry: &Uri,
        provider: &impl AsyncResourceProvider,
    ) -> Result<Vec<Output>> {
        let load_options = self.options().load_options();
        let graph = ModuleGraph::load_async(entry, &load_options, provider).await?;

        self.transpile_graph(&graph)
    }

    pub fn transpile_graph(&self, graph: &ModuleGraph) -> Result<Vec<Output>> {
        let options = self.options();
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
}
