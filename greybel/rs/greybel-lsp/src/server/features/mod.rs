#![expect(
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    reason = "Request handlers share the dispatch table's signature"
)]

mod assist;
mod edits;
mod highlight;
mod navigation;
mod ranges;
mod symbols;

use gen_lsp_types as lsp;
use greybel_core::Position;
use greybel_documents::Uri;
use greybel_type_analyzer::Analyzer;

use super::Server;
use crate::error::{ResponseError, Result};
use crate::from_lsp::{FromLsp, TryFromLsp};
use crate::to_lsp::Markup;

struct AnalyzedDocument<'a> {
    uri: Uri,
    analyzer: Analyzer<'a>,
    markup: Markup,
}

impl<'a> AnalyzedDocument<'a> {
    fn of(server: &'a Server, uri: &lsp::Uri) -> Result<Option<Self>> {
        let uri = Server::requested_uri(uri)?;

        if server.workspace.file(&uri).is_none() {
            return Ok(None);
        }

        Ok(Some(Self {
            uri,
            analyzer: server.analyzer(),
            markup: Markup {
                markdown: server.client.markdown,
                language: server.dialect.name(),
            },
        }))
    }

    fn at(
        server: &'a Server,
        params: &lsp::TextDocumentPositionParams,
    ) -> Result<Option<(Self, Position)>> {
        let document = Self::of(server, &params.text_document.uri)?;

        Ok(document.map(|document| (document, Position::from_lsp(params.position))))
    }
}

impl Server {
    fn requested_uri(uri: &lsp::Uri) -> Result<Uri> {
        Uri::try_from_lsp(uri)
            .ok_or_else(|| ResponseError::invalid_params(format!("Invalid URI: {}", uri.0)))
    }
}
