use gen_lsp_types as lsp;

use crate::semantic_tokens::SemanticTokens;
use crate::server::Server;

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::server) struct ClientSupport {
    pub configuration: bool,
    pub markdown: bool,
    pub definition_links: bool,
}

impl ClientSupport {
    pub(in crate::server) fn of(capabilities: &lsp::ClientCapabilities) -> Self {
        Self {
            configuration: capabilities
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.configuration)
                .unwrap_or(false),
            markdown: capabilities
                .text_document
                .as_ref()
                .and_then(|text_document| text_document.hover.as_ref())
                .and_then(|hover| hover.content_format.as_ref())
                .is_some_and(|formats| formats.contains(&lsp::MarkupKind::Markdown)),
            definition_links: capabilities
                .text_document
                .as_ref()
                .and_then(|text_document| text_document.definition.as_ref())
                .and_then(|definition| definition.link_support)
                .unwrap_or(false),
        }
    }
}

impl Server {
    pub(in crate::server) fn capabilities() -> lsp::ServerCapabilities {
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
                trigger_characters: triggers(&[".", "\"", "/", "@"]),
                resolve_provider: Some(true),
                ..lsp::CompletionOptions::default()
            }),
            hover_provider: Some(lsp::HoverProvider::Bool(true)),
            signature_help_provider: Some(lsp::SignatureHelpOptions {
                trigger_characters: triggers(&["(", ","]),
                ..lsp::SignatureHelpOptions::default()
            }),
            definition_provider: Some(lsp::DefinitionProvider::Bool(true)),
            type_definition_provider: Some(lsp::TypeDefinitionProvider::Bool(true)),
            references_provider: Some(lsp::ReferencesProvider::Bool(true)),
            call_hierarchy_provider: Some(lsp::CallHierarchyProvider::Bool(true)),
            document_highlight_provider: Some(lsp::DocumentHighlightProvider::Bool(true)),
            document_symbol_provider: Some(lsp::DocumentSymbolProvider::Bool(true)),
            workspace_symbol_provider: Some(lsp::WorkspaceSymbolProvider::Bool(true)),
            code_action_provider: Some(lsp::CodeActionProvider::Bool(true)),
            document_formatting_provider: Some(lsp::DocumentFormattingProvider::Bool(true)),
            folding_range_provider: Some(lsp::FoldingRangeProvider::Bool(true)),
            selection_range_provider: Some(lsp::SelectionRangeProvider::Bool(true)),
            inlay_hint_provider: Some(lsp::InlayHintProvider::Bool(true)),
            color_provider: Some(lsp::ColorProvider::Bool(true)),
            rename_provider: Some(lsp::RenameProvider::RenameOptions(lsp::RenameOptions {
                prepare_provider: Some(true),
                ..lsp::RenameOptions::default()
            })),
            semantic_tokens_provider: Some(lsp::SemanticTokensProvider::SemanticTokensOptions(
                lsp::SemanticTokensOptions {
                    legend: SemanticTokens::legend(),
                    range: None,
                    full: Some(lsp::Full::Bool(true)),
                    work_done_progress_options: lsp::WorkDoneProgressOptions::default(),
                },
            )),
            ..lsp::ServerCapabilities::default()
        }
    }
}
