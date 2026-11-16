use gen_lsp_types as lsp;
use greybel_core::Position;
use serde::{Deserialize, Serialize};

use super::{AnalyzedDocument, Server};
use crate::error::Result;
use crate::from_lsp::FromLsp;
use crate::to_lsp::ToLsp;

#[derive(Serialize, Deserialize)]
struct CompletionData {
    uri: lsp::Uri,
    position: lsp::Position,
}

impl Server {
    pub(in crate::server) fn hover(
        &mut self,
        params: lsp::HoverParams,
    ) -> Result<Option<lsp::Hover>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };

        if !self.settings.hoverdocs {
            return Ok(None);
        }

        let hover = document.analyzer.hover(&document.uri, position);

        Ok(hover.map(|hover| document.markup.hover(&hover)))
    }

    pub(in crate::server) fn completion(
        &mut self,
        params: lsp::CompletionParams,
    ) -> Result<Option<lsp::CompletionResponse>> {
        let at = params.text_document_position_params;
        let Some((document, position)) = AnalyzedDocument::at(self, &at)? else {
            return Ok(None);
        };

        if !self.settings.autocomplete {
            return Ok(None);
        }

        let data = serde_json::to_value(CompletionData {
            uri: at.text_document.uri,
            position: at.position,
        })
        .ok();
        let items = document
            .analyzer
            .completions(&document.uri, position)
            .iter()
            .map(|item| lsp::CompletionItem {
                data: item.documentation.as_ref().and(data.clone()),
                ..item.to_lsp()
            })
            .collect();

        Ok(Some(lsp::CompletionResponse::CompletionItemList(items)))
    }

    pub(in crate::server) fn completion_resolve(
        &mut self,
        mut item: lsp::CompletionItem,
    ) -> Result<lsp::CompletionItem> {
        let Some(data) = item
            .data
            .as_ref()
            .and_then(|data| CompletionData::deserialize(data).ok())
        else {
            return Ok(item);
        };
        let Some(document) = AnalyzedDocument::of(self, &data.uri)? else {
            return Ok(item);
        };
        let documentation = document
            .analyzer
            .completions(&document.uri, Position::from_lsp(data.position))
            .into_iter()
            .find(|candidate| candidate.label == item.label)
            .and_then(|candidate| candidate.documentation);

        if let Some(documentation) = documentation {
            item.documentation = Some(document.markup.documentation(&documentation));
        }

        Ok(item)
    }

    pub(in crate::server) fn signature_help(
        &mut self,
        params: lsp::SignatureHelpParams,
    ) -> Result<Option<lsp::SignatureHelp>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };
        let help = document.analyzer.signature_help(&document.uri, position);

        Ok(help.map(|help| document.markup.signature_help(&help)))
    }
}
