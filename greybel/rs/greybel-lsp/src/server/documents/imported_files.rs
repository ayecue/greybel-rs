use greybel_documents::Uri;
use rustc_hash::FxHashSet;

use crate::client_requests::ImportedFiles;
use crate::error::ResponseError;
use crate::message::{Message, Request, Response};
use crate::server::Server;

impl Server {
    /// Loads the document with its imports, then responds; a fetch through the client defers the response.
    pub(in crate::server) fn imported_files(&mut self, request: Request) -> Option<Response> {
        let params = match request.params::<ImportedFiles>() {
            Ok(params) => params,
            Err(error) => {
                return Some(Response::error(
                    Some(request.id),
                    ResponseError::invalid_params(error.to_string()),
                ));
            }
        };
        let Ok(uri) = Uri::parse(&params.uri) else {
            return Some(Response::error(
                Some(request.id),
                ResponseError::invalid_params(format!("{} isn't a URI", params.uri)),
            ));
        };

        self.fetch_handle
            .imported_file_queries
            .push((request.id, uri.clone()));
        self.load(&uri);

        None
    }

    pub(in crate::server) fn respond_imported_files(&mut self) {
        let queries = std::mem::take(&mut self.fetch_handle.imported_file_queries);

        for (id, uri) in queries {
            if self.fetch_handle.is_loading(&uri) {
                self.fetch_handle.imported_file_queries.push((id, uri));

                continue;
            }

            let uris = self.collect_imported_files(&uri);

            self.outbound
                .push(Message::Response(Response::ok::<ImportedFiles>(id, uris)));
        }
    }

    fn collect_imported_files(&self, entry: &Uri) -> Vec<String> {
        let db = self.workspace.db();
        let Some(file) = self.workspace.file(entry) else {
            return Vec::new();
        };
        let mut known_files = FxHashSet::from_iter([file]);
        let mut queue = vec![file];
        let mut uris = Vec::new();
        let mut index = 0;

        while let Some(&current) = queue.get(index) {
            index += 1;

            for &(_, target) in current.resolved_imports(db) {
                if !known_files.insert(target) || target.text(db).is_none() {
                    continue;
                }

                uris.push(target.uri(db).to_string());
                queue.push(target);
            }
        }

        uris
    }
}
