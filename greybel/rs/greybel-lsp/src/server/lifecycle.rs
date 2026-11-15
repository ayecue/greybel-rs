use gen_lsp_types as lsp;

use super::{Server, State};
use crate::error::Result;
use crate::from_lsp;
use crate::initialization::InitializationOptions;

impl Server {
    pub(super) fn initialize(
        &mut self,
        params: &lsp::InitializeParams,
    ) -> Result<lsp::InitializeResult> {
        let options = InitializationOptions::of(params.initialization_options.clone())?;

        self.dialect = options.dialect()?;
        self.workspace
            .set_options(self.settings.document_options(self.dialect));

        let capabilities = &params.capabilities;

        self.client.configuration = capabilities
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.configuration)
            .unwrap_or(false);
        self.client.markdown = capabilities
            .text_document
            .as_ref()
            .and_then(|text_document| text_document.hover.as_ref())
            .and_then(|hover| hover.content_format.as_ref())
            .is_some_and(|formats| formats.contains(&lsp::MarkupKind::Markdown));

        let folders = match &params.workspace_folders_initialize_params.workspace_folders {
            Some(lsp::WorkspaceFolders::WorkspaceFolderList(folders)) => {
                folders.iter().map(|folder| folder.uri.clone()).collect()
            }
            #[allow(deprecated)]
            _ => params.root_uri.iter().cloned().collect::<Vec<_>>(),
        };

        self.folders = folders.iter().filter_map(from_lsp::uri).collect();
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

    pub(super) fn discover_workspace(&mut self) {
        if self.client.configuration {
            self.request_configuration();
        } else {
            self.find_files();
        }
    }

    pub(super) fn shutdown(&mut self, (): ()) {
        self.state = State::Teardown;
    }

    pub(super) fn exit(&mut self) {
        self.state = State::Exit(i32::from(self.state != State::Teardown));
    }

    fn capabilities() -> lsp::ServerCapabilities {
        let triggers =
            |characters: &[&str]| Some(characters.iter().map(|&c| c.to_owned()).collect());

        lsp::ServerCapabilities {
            position_encoding: Some(lsp::PositionEncodingKind::UTF16),
            text_document_sync: Some(lsp::TextDocumentSync::Options(
                lsp::TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(lsp::TextDocumentSyncKind::Incremental),
                    ..lsp::TextDocumentSyncOptions::default()
                },
            )),
            completion_provider: Some(lsp::CompletionOptions {
                trigger_characters: triggers(&[".", "\"", "/"]),
                resolve_provider: Some(true),
                ..lsp::CompletionOptions::default()
            }),
            hover_provider: Some(lsp::HoverProvider::Bool(true)),
            signature_help_provider: Some(lsp::SignatureHelpOptions {
                trigger_characters: triggers(&["(", ","]),
                ..lsp::SignatureHelpOptions::default()
            }),
            definition_provider: Some(lsp::DefinitionProvider::Bool(true)),
            references_provider: Some(lsp::ReferencesProvider::Bool(true)),
            document_highlight_provider: Some(lsp::DocumentHighlightProvider::Bool(true)),
            document_symbol_provider: Some(lsp::DocumentSymbolProvider::Bool(true)),
            workspace_symbol_provider: Some(lsp::WorkspaceSymbolProvider::Bool(true)),
            code_action_provider: Some(lsp::CodeActionProvider::Bool(true)),
            document_formatting_provider: Some(lsp::DocumentFormattingProvider::Bool(true)),
            rename_provider: Some(lsp::RenameProvider::RenameOptions(lsp::RenameOptions {
                prepare_provider: Some(true),
                ..lsp::RenameOptions::default()
            })),
            semantic_tokens_provider: Some(lsp::SemanticTokensProvider::SemanticTokensOptions(
                lsp::SemanticTokensOptions {
                    legend: crate::semantic_tokens::legend(),
                    range: None,
                    full: Some(lsp::Full::Bool(true)),
                    work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                },
            )),
            ..lsp::ServerCapabilities::default()
        }
    }
}
