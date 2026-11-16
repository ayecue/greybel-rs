use gen_lsp_types as lsp;
use greybel_documents::Uri;

use super::ClientSupport;
use crate::error::Result;
use crate::from_lsp::TryFromLsp;
use crate::initialization::InitializationOptions;
use crate::server::{Server, State};

impl Server {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Request handlers share the dispatch table's signature"
    )]
    pub(in crate::server) fn initialize(
        &mut self,
        params: lsp::InitializeParams,
    ) -> Result<lsp::InitializeResult> {
        let options = InitializationOptions::of(params.initialization_options.clone())?;

        self.dialect = options.dialect()?;
        self.workspace
            .set_options(self.settings.document_options(self.dialect));
        self.client = ClientSupport::of(&params.capabilities);
        self.folders = Self::workspace_folders(&params);
        self.access
            .set_resolution(self.settings.resolution(&self.folders));
        self.state = State::Active;

        Ok(lsp::InitializeResult {
            capabilities: Self::capabilities(),
            server_info: Some(lsp::ServerInfo {
                name: Self::NAME.to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    pub(in crate::server) fn discover_workspace(&mut self) {
        if self.client.configuration {
            self.request_configuration();
        } else {
            self.find_files();
        }
    }

    pub(in crate::server) fn shutdown(&mut self) {
        self.state = State::Teardown;
    }

    pub(in crate::server) fn exit(&mut self) {
        self.state = State::Exit(i32::from(self.state != State::Teardown));
    }

    fn workspace_folders(params: &lsp::InitializeParams) -> Vec<Uri> {
        let folders = match &params.workspace_folders_initialize_params.workspace_folders {
            Some(lsp::WorkspaceFolders::WorkspaceFolderList(folders)) => {
                folders.iter().map(|folder| folder.uri.clone()).collect()
            }
            #[allow(deprecated)]
            _ => params.root_uri.iter().cloned().collect::<Vec<_>>(),
        };

        folders.iter().filter_map(Uri::try_from_lsp).collect()
    }
}
