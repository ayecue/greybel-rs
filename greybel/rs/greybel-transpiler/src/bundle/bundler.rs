use rustc_hash::FxHashSet;

use super::output::{Output, Part, PendingOutput};
use super::template::{HEADER, MODULE, Template, emit_boilerplate, emit_multiline_string_function};
use crate::documents::{ImportKind, ModuleGraph, ModuleId};
use crate::emit::{ConstantTable, EmitFlags, EmitState, Emitter};
use crate::error::Result;
use crate::names::{Charset, NameTable};
use crate::options::TranspileOptions;

pub(super) struct Bundler<'a, E> {
    emitter: E,
    graph: &'a ModuleGraph,
    options: &'a TranspileOptions,
    template: Template,
    namespaces: Vec<String>,
    /// The `const enum` entries of every module.
    constants: ConstantTable,
    emitted_native_imports: FxHashSet<ModuleId>,
    wrapper_modules: FxHashSet<ModuleId>,
}

impl<'a, E: Emitter> Bundler<'a, E> {
    pub(super) fn new(
        mut emitter: E,
        graph: &'a ModuleGraph,
        options: &'a TranspileOptions,
    ) -> Result<Self> {
        let template = Template::new(emit_boilerplate(&mut emitter, MODULE, options)?);

        Ok(Self {
            emitter,
            graph,
            options,
            template,
            namespaces: module_namespaces(graph, options.obfuscation),
            constants: ConstantTable::of(graph.ids().map(|id| &*graph.module(id).ast)),
            emitted_native_imports: FxHashSet::default(),
            wrapper_modules: FxHashSet::default(),
        })
    }

    pub(super) fn outputs(mut self, main: ModuleId) -> Result<Vec<Output>> {
        let graph = self.graph;
        let natives = graph.native_imports(main);
        let main_modules = self.wrap_modules(main)?;
        let native_modules = natives
            .iter()
            .map(|&native| self.wrap_modules(native))
            .collect::<Result<Vec<_>>>()?;
        let has_modules =
            !main_modules.is_empty() || native_modules.iter().any(|modules| !modules.is_empty());
        let header = Part::plain(emit_boilerplate(&mut self.emitter, HEADER, self.options)?);
        let multiline_string = emit_multiline_string_function(
            &mut self.emitter,
            graph.dependency_order(main).into_iter(),
            self.options,
        )?;

        let mut output = PendingOutput::default();

        if let Some(function) = multiline_string {
            output.push(&Part::plain(function));
        }

        let preamble = self.emitter.preamble(&EmitState::for_module(
            graph,
            &self.namespaces,
            main,
            self.options,
        ));

        for line in preamble {
            output.push(&Part::plain(line));
        }

        if has_modules {
            output.push(&header);
        }

        for module in &main_modules {
            output.push(module);
        }

        output.push(&self.emit_module(main)?);

        let mut outputs = vec![output.finish(graph.module(main).uri.clone(), graph)];

        for (&native, modules) in natives.iter().zip(&native_modules) {
            let mut output = PendingOutput::default();

            for module in modules {
                output.push(module);
            }

            output.push(&self.emit_module(native)?);
            outputs.push(output.finish(graph.module(native).uri.clone(), graph));
        }

        Ok(outputs)
    }

    fn wrap_modules(&mut self, root: ModuleId) -> Result<Vec<Part>> {
        let mut modules = Vec::new();

        self.collect(root, root, &mut modules)?;

        Ok(modules)
    }

    fn collect(&mut self, root: ModuleId, id: ModuleId, modules: &mut Vec<Part>) -> Result<()> {
        if self.wrapper_modules.contains(&id) {
            return Ok(());
        }

        let graph = self.graph;
        let module = graph.module(id);

        if id != root && module.is_imported() {
            self.wrapper_modules.insert(id);

            let code = self.emit_module(id)?;

            modules.push(self.template.fill(&self.namespaces[id.index()], &code));
        }

        for edge in module.dependencies() {
            if edge.kind != ImportKind::NativeImport {
                self.collect(root, edge.module, modules)?;
            }
        }

        Ok(())
    }

    fn emit_module(&mut self, id: ModuleId) -> Result<Part> {
        let state = EmitState::for_module(self.graph, &self.namespaces, id, self.options)
            .with_constants(&self.constants);
        let mut state =
            state.with_emitted_native_imports(std::mem::take(&mut self.emitted_native_imports));
        let root = state.ast().root();
        let result = self.emitter.emit(&mut state, root, EmitFlags::default());
        let (output, native_imports) = state.into_parts();

        self.emitted_native_imports = native_imports;
        result?;

        let origins = output.origins().to_vec();

        Ok(Part::new(self.emitter.finish(output), &origins))
    }
}

fn module_namespaces(graph: &ModuleGraph, obfuscation: bool) -> Vec<String> {
    let mut names = NameTable::new(Charset::modules(obfuscation).chars());

    graph
        .ids()
        .map(|id| {
            let label = graph.module(id).uri.label();

            names.create(&label).unwrap_or(&label).to_owned()
        })
        .collect()
}
