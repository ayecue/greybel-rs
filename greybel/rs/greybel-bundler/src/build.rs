use greybel_core::Dialect;
use greybel_documents::{
    AsyncResourceProvider, ModuleGraph, ModuleId, ResourceProvider, Uri, Workspace,
};
use greybel_transpiler::{PathMapping, TranspileOptions, Transpiler};
use vscode_uri::UriChange;

use crate::destination::Destination;
use crate::error::{BuildError, Result};
use crate::options::BuildOptions;
use crate::output::{BuildOutput, OutputFile};

#[derive(Debug, Clone)]
pub struct Build {
    options: BuildOptions,
    destination: Destination,
}

impl Build {
    const GENERAL: &str = "general";
    const PARAMS: &str = "params";

    #[must_use]
    pub fn new(options: BuildOptions, rng: &mut fastrand::Rng) -> Self {
        let destination = Destination::new(&options.ingame_directory, rng);

        Self {
            options,
            destination,
        }
    }

    #[must_use]
    pub fn destination(&self) -> &Destination {
        &self.destination
    }

    pub fn run(&self, entries: &[Uri], provider: &impl ResourceProvider) -> Result<BuildOutput> {
        self.check(entries)?;

        let mut workspace = Workspace::new(self.options.transpile.load_options());

        for entry in entries {
            workspace.load(entry, provider)?;
        }

        self.finish(&workspace, entries)
    }

    pub async fn run_async(
        &self,
        entries: &[Uri],
        provider: &impl AsyncResourceProvider,
    ) -> Result<BuildOutput> {
        self.check(entries)?;

        let mut workspace = Workspace::new(self.options.transpile.load_options());

        for entry in entries {
            workspace.load_async(entry, provider).await?;
        }

        self.finish(&workspace, entries)
    }

    fn check(&self, entries: &[Uri]) -> Result<()> {
        if entries.is_empty() {
            return Err(BuildError::NoEntries);
        }

        if self.options.output_name.is_some() && entries.len() != 1 {
            return Err(BuildError::OutputNameWithEntries(entries.len()));
        }

        Ok(())
    }

    fn finish(&self, workspace: &Workspace, entries: &[Uri]) -> Result<BuildOutput> {
        let graphs = entries
            .iter()
            .map(|entry| workspace.module_graph(entry))
            .collect::<Result<Vec<_>, _>>()?;
        let root = CommonFolder::of(graphs.iter().flat_map(Self::output_modules))?;
        let mapping = PathMapping {
            local_root: root.clone(),
            ingame_directory: if self.options.auto_compile.is_some() {
                self.destination.resources.clone()
            } else {
                self.destination.directory.clone()
            },
        };
        let options = self.transpile_options(mapping.clone());
        let transpiler = Transpiler::new(&options);
        let mut files: Vec<OutputFile> = Vec::new();
        let mut entry_paths = Vec::new();

        for graph in &graphs {
            let outputs = transpiler.transpile_graph(graph)?;

            for (index, output) in outputs.into_iter().enumerate() {
                let mut path = mapping
                    .relative(&output.target)
                    .unwrap_or_else(|| output.target.label());

                if index == 0 {
                    if let Some(name) = &self.options.output_name {
                        path = Self::replace_file_name(&path, name);
                    }

                    entry_paths.push(path.clone());
                }

                let file = OutputFile {
                    source: output.target,
                    path,
                    code: output.code,
                };

                match files.iter_mut().find(|entry| entry.path == file.path) {
                    Some(entry) => *entry = file,
                    None => files.push(file),
                }
            }
        }

        Ok(BuildOutput {
            root,
            files,
            entries: entry_paths,
            destination: self.destination.clone(),
            auto_compile: self.options.auto_compile,
        })
    }

    fn output_modules(graph: &ModuleGraph) -> impl Iterator<Item = &Uri> {
        let main = graph.entry();
        let natives = main.map_or(&[][..], |main| graph.native_imports(main));

        main.into_iter()
            .chain(natives.iter().copied())
            .map(|id: ModuleId| &graph.module(id).uri)
    }

    fn transpile_options(&self, mapping: PathMapping) -> TranspileOptions {
        TranspileOptions {
            path_mapping: Some(mapping),
            excluded_names: Self::excluded_names(&self.options.transpile),
            ..self.options.transpile.clone()
        }
    }

    /// Collects names that renaming must preserve: `options.excluded_names`, `params`, and all general API definitions.
    pub(crate) fn excluded_names(options: &TranspileOptions) -> Vec<String> {
        let api = match options.dialect {
            Dialect::MiniScript => miniscript_meta::container(),
            Dialect::Greybel | Dialect::GreyScript => greyscript_meta::container(),
        };
        let general = api
            .signature(Self::GENERAL)
            .into_iter()
            .flat_map(|signature| signature.definitions().keys().cloned());
        let mut excluded_names = options.excluded_names.clone();

        excluded_names.push(Self::PARAMS.to_owned());
        excluded_names.extend(general);

        excluded_names
    }

    fn replace_file_name(path: &str, name: &str) -> String {
        match path.rsplit_once('/') {
            Some((folder, _)) => format!("{folder}/{name}"),
            None => name.to_owned(),
        }
    }
}

struct CommonFolder;

impl CommonFolder {
    fn of<'a>(mut uris: impl Iterator<Item = &'a Uri>) -> Result<Uri> {
        let Some(first) = uris.next() else {
            return Err(BuildError::NoEntries);
        };
        let mut segments: Vec<&str> = Self::folder_segments(first);

        for uri in uris {
            if uri.scheme() != first.scheme() || uri.authority() != first.authority() {
                return Err(BuildError::NoCommonFolder(first.clone(), uri.clone()));
            }

            let other = Self::folder_segments(uri);
            let common = segments
                .iter()
                .zip(&other)
                .take_while(|(left, right)| left == right)
                .count();

            segments.truncate(common);
        }

        let path = format!("/{}", segments.join("/"));

        first
            .with(UriChange {
                path: Some(path),
                query: Some(String::new()),
                fragment: Some(String::new()),
                ..UriChange::default()
            })
            .map_err(|_| BuildError::NoCommonFolder(first.clone(), first.clone()))
    }

    fn folder_segments(uri: &Uri) -> Vec<&str> {
        let path = uri.path();
        let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);

        folder
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    }
}
