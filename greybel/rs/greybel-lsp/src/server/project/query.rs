use std::collections::BTreeMap;

use greybel_documents::Uri;
use greybel_project::{Manifest, Project, ProjectError};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::client_requests::ProjectResult;
use crate::message::RequestId;

pub(in crate::server) struct ProjectQuery {
    pub id: RequestId,
    document: Uri,
    folder: Uri,
    fallback: Manifest,
    profile: Option<String>,
    project: Option<Project>,
    texts: FxHashMap<Uri, Option<String>>,
    pending_reads: FxHashSet<Uri>,
}

pub(in crate::server) enum Progress {
    Read(Vec<Uri>),
    Wait,
    Complete(Box<Result<ProjectResult, ProjectError>>),
}

impl ProjectQuery {
    pub(in crate::server) fn new(
        id: RequestId,
        document: Uri,
        folder: Uri,
        fallback: Manifest,
        profile: Option<String>,
    ) -> Self {
        Self {
            id,
            document,
            folder,
            fallback,
            profile,
            project: None,
            texts: FxHashMap::default(),
            pending_reads: FxHashSet::default(),
        }
    }

    pub(in crate::server) fn waits_for(&self, uri: &Uri) -> bool {
        self.pending_reads.contains(uri) && !self.texts.contains_key(uri)
    }

    pub(in crate::server) fn receive(&mut self, uri: &Uri, text: Option<String>) {
        self.texts.insert(uri.clone(), text);
    }

    pub(in crate::server) fn progress(&mut self) -> Progress {
        if self.project.is_none() {
            match self.locate() {
                Ok(Some(project)) => match project.with_profile(self.profile.as_deref()) {
                    Ok(project) => self.project = Some(project),
                    Err(error) => return Progress::Complete(Box::new(Err(error))),
                },
                Ok(None) => return self.request_unread(Project::locations(&self.document)),
                Err(error) => return Progress::Complete(Box::new(Err(error))),
            }
        }

        let project = self.project.as_ref().expect("the project is found");
        let manifest = &project.manifest;
        let files: Vec<Uri> = [&manifest.build.variables, &manifest.run.variables]
            .into_iter()
            .flat_map(|variables| project.variable_files(&self.folder, variables))
            .collect();

        if files.iter().any(|file| !self.texts.contains_key(file)) {
            return self.request_unread(files);
        }

        Progress::Complete(Box::new(Ok(ProjectResult {
            root: project.root.as_ref().map(ToString::to_string),
            manifest: manifest.clone(),
            build_variables: self.variables(project, &manifest.build.variables),
            run_variables: self.variables(project, &manifest.run.variables),
        })))
    }

    fn locate(&self) -> Result<Option<Project>, ProjectError> {
        for location in Project::locations(&self.document) {
            match self.texts.get(&location) {
                None => return Ok(None),
                Some(Some(text)) => return Project::read(&location, text).map(Some),
                Some(None) => {}
            }
        }

        Ok(Some(Project {
            root: None,
            manifest: self.fallback.clone(),
        }))
    }

    fn request_unread(&mut self, files: Vec<Uri>) -> Progress {
        let unread: Vec<Uri> = files
            .into_iter()
            .filter(|file| {
                !self.texts.contains_key(file) && self.pending_reads.insert(file.clone())
            })
            .collect();

        if unread.is_empty() {
            Progress::Wait
        } else {
            Progress::Read(unread)
        }
    }

    fn variables(
        &self,
        project: &Project,
        variables: &greybel_project::VariableSettings,
    ) -> BTreeMap<String, String> {
        project.read_variables(&self.folder, variables, |file| {
            self.texts.get(file).cloned().flatten()
        })
    }
}

#[derive(Default)]
pub(in crate::server) struct ProjectTable {
    pub queries: Vec<ProjectQuery>,
}
