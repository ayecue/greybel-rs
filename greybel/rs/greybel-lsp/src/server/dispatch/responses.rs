use gen_lsp_types as lsp;
use greybel_documents::Uri;

use crate::files::{FileSearch, ReadFile};
use crate::message::Response;
use crate::server::Server;
use crate::server::state::Expectation;

impl Server {
    pub(in crate::server) fn response(&mut self, response: &Response) {
        let Some(expectation) = response
            .id
            .as_ref()
            .and_then(|id| self.outbound.resolve(id))
        else {
            return;
        };

        match expectation {
            Expectation::Configuration => {
                let section = response
                    .result::<lsp::ConfigurationRequest>()
                    .and_then(|values| values.into_iter().next())
                    .filter(|value| !value.is_null());

                self.apply_settings(section);
                self.find_files();
            }
            Expectation::FileSearch => {
                let uris = response.result::<FileSearch>().unwrap_or_default();

                self.handle_find_files(
                    uris.iter().filter_map(|uri| Uri::parse(uri).ok()).collect(),
                );
            }
            Expectation::Read(uri) => {
                let content = response.result::<ReadFile>().flatten();

                self.handle_file_read(&uri, content);
            }
            Expectation::ProjectFile(uri) => {
                let content = response.result::<ReadFile>().flatten();

                self.handle_project_file(&uri, content);
            }
        }
    }
}
