use greybel_documents::Uri;
use greybel_project::Manifest;

use super::Server;
use super::state::projects::Progress;
use super::state::{Expectation, FileAccess, ProjectQuery};
use crate::error::ResponseError;
use crate::files::{Project, ProjectRequest, ReadFile, ReadFileRequest};
use crate::message::{Message, Request, Response};

impl Server {
    /// Starts a project request and responds after reading required files: immediately for local files or as client
    /// file responses arrive.
    pub(super) fn project(&mut self, request: Request) -> Option<Response> {
        let params = match request.params::<Project>() {
            Ok(params) => params,
            Err(error) => {
                return Some(Response::error(
                    Some(request.id),
                    ResponseError::invalid_params(error.to_string()),
                ));
            }
        };
        let ProjectRequest {
            uri,
            fallback,
            profile,
        } = params;
        let Ok(document) = Uri::parse(&uri) else {
            return Some(Response::error(
                Some(request.id),
                ResponseError::invalid_params(format!("{uri} isn't a URI")),
            ));
        };
        let folder = self.folder_of(&document);
        let query = ProjectQuery::new(
            request.id,
            document,
            folder,
            fallback.unwrap_or_else(Manifest::default),
            profile,
        );

        self.advance(query)
    }

    /// Accepts client-provided text for a requested file and completes any project queries unblocked by it.
    pub(super) fn handle_project_file(&mut self, uri: &Uri, text: Option<&String>) {
        let queries = std::mem::take(&mut self.projects.queries);

        for mut query in queries {
            if query.waits_for(uri) {
                query.receive(uri, text.cloned());
            }

            if let Some(response) = self.advance(query) {
                self.outbound.push(Message::Response(response));
            }
        }
    }

    /// Advances `query` and returns its response when complete, or `None` while client files are pending.
    fn advance(&mut self, mut query: ProjectQuery) -> Option<Response> {
        loop {
            match query.progress() {
                Progress::Complete(result) => {
                    return Some(match *result {
                        Ok(project) => Response::ok::<Project>(query.id, project),
                        Err(error) => Response::error(
                            Some(query.id),
                            ResponseError::invalid_params(error.to_string()),
                        ),
                    });
                }
                Progress::Wait => {
                    self.projects.queries.push(query);

                    return None;
                }
                Progress::Read(files) => match &self.access {
                    FileAccess::Client(_) => {
                        for file in files {
                            self.outbound.request::<ReadFile>(
                                ReadFileRequest {
                                    uri: file.to_string(),
                                },
                                Expectation::ProjectFile(file),
                            );
                        }

                        self.projects.queries.push(query);

                        return None;
                    }
                    #[cfg(not(target_family = "wasm"))]
                    FileAccess::Local(_) => {
                        for file in files {
                            let text = greybel_documents_fs_provider::FsProvider::text(&file);

                            query.receive(&file, text);
                        }
                    }
                },
            }
        }
    }

    /// Returns `document`'s workspace folder, falling back to its containing folder.
    fn folder_of(&self, document: &Uri) -> Uri {
        let path = document.path();

        self.folders
            .iter()
            .filter(|folder| {
                folder.scheme() == document.scheme()
                    && path.starts_with(&format!("{}/", folder.path().trim_end_matches('/')))
            })
            .max_by_key(|folder| folder.path().len())
            .cloned()
            .unwrap_or_else(|| document.dirname().unwrap_or_else(|_| document.clone()))
    }
}
