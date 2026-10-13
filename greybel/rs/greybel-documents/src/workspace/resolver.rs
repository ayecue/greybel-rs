use rustc_hash::FxHashMap;
use vscode_uri::Uri;

use super::Workspace;
use crate::error::Result;
use crate::provider::{AsyncResourceProvider, ResourceProvider, ResourceRequest, ResourceResponse};
use crate::store::{Db, SourceFile};

struct Reference {
    origin: Option<SourceFile>,
    target: String,
    is_dependency: bool,
}

struct Read {
    uri: Uri,
    version: Option<u32>,
    references: Vec<Reference>,
}

#[derive(Default)]
struct ReadTable(Vec<Read>);

enum State {
    Start,
    Walk,
    Lookup(Vec<Reference>),
    Fetch(ReadTable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Requests(Vec<ResourceRequest>),
    Complete,
}

impl ReadTable {
    fn add<DB: Db>(&mut self, workspace: &Workspace<DB>, uri: Uri, reference: Reference) {
        match self.0.iter_mut().find(|read| read.uri == uri) {
            Some(read) => read.references.push(reference),
            None => self.0.push(Read {
                version: workspace.version(&uri),
                uri,
                references: vec![reference],
            }),
        }
    }
}

pub struct DependencyResolver {
    entry: Uri,
    state: State,
    reference_queue: Vec<Reference>,
    file_versions: FxHashMap<SourceFile, u32>,
}

impl DependencyResolver {
    #[must_use]
    pub fn new(entry: Uri) -> Self {
        Self {
            entry,
            state: State::Start,
            reference_queue: Vec::new(),
            file_versions: FxHashMap::default(),
        }
    }

    pub fn resolve<DB: Db>(
        mut self,
        workspace: &mut Workspace<DB>,
        provider: &impl ResourceProvider,
    ) -> Result<()> {
        while let Resolution::Requests(requests) = self.next(workspace) {
            let responses = ResourceRequest::run_all(requests, provider)?;

            self.process_responses(workspace, responses);
        }

        Ok(())
    }

    pub async fn resolve_async<DB: Db>(
        mut self,
        workspace: &mut Workspace<DB>,
        provider: &impl AsyncResourceProvider,
    ) -> Result<()> {
        while let Resolution::Requests(requests) = self.next(workspace) {
            let responses = ResourceRequest::run_all_async(requests, provider).await?;

            self.process_responses(workspace, responses);
        }

        Ok(())
    }

    pub fn next<DB: Db>(&mut self, workspace: &Workspace<DB>) -> Resolution {
        if self.is_outdated(workspace) {
            self.restart();
        }

        if matches!(self.state, State::Start) {
            self.start(workspace);
        }

        if matches!(self.state, State::Walk) {
            self.walk(workspace);
        }

        let db = workspace.db();

        match &self.state {
            State::Start | State::Walk => Resolution::Complete,
            State::Lookup(references) => Resolution::Requests(
                references
                    .iter()
                    .filter_map(|reference| {
                        Some(ResourceRequest::RelativeTo {
                            source: reference.origin?.uri(db).clone(),
                            target: reference.target.clone(),
                        })
                    })
                    .collect(),
            ),
            State::Fetch(reads) => Resolution::Requests(
                reads
                    .0
                    .iter()
                    .map(|read| ResourceRequest::Read(read.uri.clone()))
                    .collect(),
            ),
        }
    }

    pub fn process_responses<DB: Db>(
        &mut self,
        workspace: &mut Workspace<DB>,
        responses: Vec<ResourceResponse>,
    ) {
        let request_count = match &self.state {
            State::Start | State::Walk => 0,
            State::Lookup(references) => references.len(),
            State::Fetch(reads) => reads.0.len(),
        };

        assert_eq!(
            responses.len(),
            request_count,
            "one response per request, in the order requested"
        );
        if self.is_outdated(workspace) {
            self.restart();

            return;
        }

        match std::mem::replace(&mut self.state, State::Walk) {
            State::Start => self.state = State::Start,
            State::Walk => {}
            State::Lookup(references) => self.process_uris(workspace, references, responses),
            State::Fetch(reads) => self.process_contents(workspace, reads, responses),
        }
    }

    fn is_outdated<DB: Db>(&self, workspace: &Workspace<DB>) -> bool {
        let db = workspace.db();

        self.file_versions
            .iter()
            .any(|(file, &version)| file.version(db) != version)
    }

    fn restart(&mut self) {
        self.state = State::Start;
        self.reference_queue.clear();
        self.file_versions.clear();
    }

    fn start<DB: Db>(&mut self, workspace: &Workspace<DB>) {
        let reference = Reference {
            origin: None,
            target: self.entry.to_string(),
            is_dependency: true,
        };

        self.state = State::Walk;

        match workspace.file(&self.entry) {
            Some(file) if file.text(workspace.db()).is_some() => {
                self.visit(workspace, &reference, file);
            }
            _ => {
                let mut reads = ReadTable::default();

                reads.add(workspace, self.entry.clone(), reference);
                self.state = State::Fetch(reads);
            }
        }
    }

    fn walk<DB: Db>(&mut self, workspace: &Workspace<DB>) {
        while !self.reference_queue.is_empty() {
            let mut lookups = Vec::new();

            for reference in std::mem::take(&mut self.reference_queue) {
                let db = workspace.db();
                let link_target = reference
                    .origin
                    .and_then(|origin| origin.links(db).get(&reference.target).copied())
                    .filter(|file| file.text(db).is_some());

                match link_target {
                    Some(file) => self.visit(workspace, &reference, file),
                    None => lookups.push(reference),
                }
            }

            if !lookups.is_empty() {
                self.state = State::Lookup(lookups);

                return;
            }
        }
    }

    fn process_uris<DB: Db>(
        &mut self,
        workspace: &mut Workspace<DB>,
        references: Vec<Reference>,
        responses: Vec<ResourceResponse>,
    ) {
        let mut reads = ReadTable::default();

        for (reference, response) in references.into_iter().zip(responses) {
            let ResourceResponse::Uri(uri) = response else {
                panic!("the response to a resolution is a URI")
            };

            match workspace.file(&uri) {
                Some(file) if file.text(workspace.db()).is_some() => {
                    self.link(workspace, &reference, file);
                }
                _ => reads.add(workspace, uri, reference),
            }
        }

        if !reads.0.is_empty() {
            self.state = State::Fetch(reads);
        }
    }

    fn process_contents<DB: Db>(
        &mut self,
        workspace: &mut Workspace<DB>,
        reads: ReadTable,
        responses: Vec<ResourceResponse>,
    ) {
        for (read, response) in reads.0.into_iter().zip(responses) {
            let ResourceResponse::Content(content) = response else {
                panic!("the response to a read is the content")
            };
            let file = match workspace.file(&read.uri) {
                Some(file) if Some(file.version(workspace.db())) != read.version => file,
                _ => workspace.store(&read.uri, content.map(Into::into)),
            };

            for reference in read.references {
                self.link(workspace, &reference, file);
            }
        }
    }

    fn link<DB: Db>(
        &mut self,
        workspace: &mut Workspace<DB>,
        reference: &Reference,
        file: SourceFile,
    ) {
        if let Some(origin) = reference.origin {
            workspace.link(origin, &reference.target, file);
        }

        self.visit(workspace, reference, file);
    }

    fn visit<DB: Db>(
        &mut self,
        workspace: &Workspace<DB>,
        reference: &Reference,
        file: SourceFile,
    ) {
        let db = workspace.db();

        if !reference.is_dependency
            || file.text(db).is_none()
            || self.file_versions.contains_key(&file)
        {
            return;
        }

        self.file_versions.insert(file, file.version(db));
        for import in file.imports(db) {
            self.reference_queue.push(Reference {
                origin: Some(file),
                target: import.target.clone(),
                is_dependency: import.kind.is_dependency(),
            });
        }
    }
}
