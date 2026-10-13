mod assist;
mod edits;
mod highlight;
mod navigation;
mod symbols;

use gen_lsp_types as lsp;
use greybel_documents::Uri;
use greybel_type_analyzer::Analyzer;

use super::Server;
use crate::error::{ResponseError, Result};
use crate::from_lsp;
use crate::to_lsp::Markup;

struct AnalyzedDocument<'a> {
    uri: Uri,
    analyzer: Analyzer<'a>,
    markup: Markup,
}

impl Server {
    fn requested_uri(uri: &lsp::Uri) -> Result<Uri> {
        from_lsp::uri(uri)
            .ok_or_else(|| ResponseError::invalid_params(format!("Invalid URI: {}", uri.0)))
    }

    fn document_analysis(&self, uri: &lsp::Uri) -> Result<Option<AnalyzedDocument<'_>>> {
        let uri = Self::requested_uri(uri)?;

        if self.workspace.file(&uri).is_none() {
            return Ok(None);
        }

        Ok(Some(AnalyzedDocument {
            uri,
            analyzer: self.analyzer(),
            markup: Markup {
                markdown: self.client.markdown,
                language: self.dialect.name(),
            },
        }))
    }
}
