mod check;
mod resolver;

use std::sync::Arc;

use check::Check;
pub use resolver::{DependencyResolver, Resolution};
use salsa::Setter;
use vscode_uri::Uri;

use crate::error::{DocumentError, Result};
use crate::graph::ModuleGraph;
use crate::options::DocumentOptions;
use crate::provider::{AsyncResourceProvider, ResourceProvider};
use crate::store::{Db, DocumentDatabase, Project, SourceFile, SourceFileTable};

pub struct Workspace<DB = DocumentDatabase> {
    db: DB,
    project: Project,
    files: SourceFileTable,
}

impl Workspace {
    #[must_use]
    pub fn new(options: DocumentOptions) -> Self {
        Self::with_database(DocumentDatabase::default(), options)
    }
}

impl<DB: Db> Workspace<DB> {
    pub fn with_database(db: DB, options: DocumentOptions) -> Self {
        let project = Project::new(&db, options);

        Self {
            db,
            project,
            files: SourceFileTable::default(),
        }
    }

    #[must_use]
    pub fn db(&self) -> &DB {
        &self.db
    }

    pub fn db_mut(&mut self) -> &mut DB {
        &mut self.db
    }

    #[must_use]
    pub fn project(&self) -> Project {
        self.project
    }

    #[must_use]
    pub fn options(&self) -> &DocumentOptions {
        self.project.options(&self.db)
    }

    pub fn set_options(&mut self, options: DocumentOptions) {
        if *self.options() != options {
            self.project.set_options(&mut self.db).to(options);
        }
    }

    #[must_use]
    pub fn file(&self, uri: &Uri) -> Option<SourceFile> {
        self.files.get(uri)
    }

    pub fn files(&self) -> impl Iterator<Item = SourceFile> + '_ {
        self.files.iter()
    }

    #[must_use]
    pub fn version(&self, uri: &Uri) -> Option<u32> {
        Some(self.file(uri)?.version(&self.db))
    }

    #[must_use]
    pub fn text(&self, uri: &Uri) -> Option<Arc<str>> {
        self.file(uri)?.text(&self.db).clone()
    }

    pub fn set_file(&mut self, uri: &Uri, text: impl Into<Arc<str>>) {
        self.store(uri, Some(text.into()));
    }

    pub fn remove_file(&mut self, uri: &Uri) -> bool {
        let exists = self.text(uri).is_some();

        self.store(uri, None);

        exists
    }

    #[must_use]
    pub fn dependents(&self, uri: &Uri) -> Vec<Uri> {
        let Some(file) = self.file(uri) else {
            return Vec::new();
        };
        let mut dependents: Vec<Uri> = self
            .files()
            .filter(|origin| {
                origin.text(&self.db).is_some()
                    && origin
                        .resolved_imports(&self.db)
                        .iter()
                        .any(|&(_, target)| target == file)
            })
            .map(|origin| origin.uri(&self.db).clone())
            .collect();

        dependents.sort();

        dependents
    }

    pub fn load(&mut self, entry: &Uri, provider: &impl ResourceProvider) -> Result<()> {
        DependencyResolver::new(entry.clone()).resolve(self, provider)
    }

    pub async fn load_async(
        &mut self,
        entry: &Uri,
        provider: &impl AsyncResourceProvider,
    ) -> Result<()> {
        DependencyResolver::new(entry.clone())
            .resolve_async(self, provider)
            .await
    }

    pub fn module_graph(&self, entry: &Uri) -> Result<ModuleGraph> {
        let Some(file) = self.file(entry) else {
            return Err(DocumentError::MissingTarget {
                target: entry.clone(),
            });
        };

        if let Some(error) = Check::new(&self.db).first_error(file) {
            return Err(error);
        }

        Ok(ModuleGraph::build(&self.db, file))
    }

    fn store(&mut self, uri: &Uri, text: Option<Arc<str>>) -> SourceFile {
        self.files.store(&mut self.db, self.project, uri, text)
    }

    fn link(&mut self, origin: SourceFile, target: &str, file: SourceFile) {
        origin.link(&mut self.db, target, file);
    }
}
