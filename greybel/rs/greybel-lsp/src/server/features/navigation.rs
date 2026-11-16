use gen_lsp_types as lsp;
use greybel_core::Position;

use super::{AnalyzedDocument, Server};
use crate::error::{ResponseError, Result};
use crate::from_lsp::FromLsp;
use crate::to_lsp::{DefinitionLink, DocumentHighlight, IncomingCall, OutgoingCall, ToLsp};

impl Server {
    pub(in crate::server) fn definition(
        &mut self,
        params: lsp::DefinitionParams,
    ) -> Result<Option<lsp::DefinitionResponse>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };
        let definitions = document.analyzer.definition(&document.uri, position);

        if definitions.is_empty() {
            return Ok(None);
        }

        let origin = self
            .client
            .definition_links
            .then(|| document.analyzer.definition_origin(&document.uri, position))
            .flatten();
        let Some(origin) = origin else {
            let locations = definitions.iter().map(ToLsp::to_lsp).collect();

            return Ok(Some(lsp::DefinitionResponse::Definition(
                lsp::Definition::LocationList(locations),
            )));
        };
        let links = definitions
            .iter()
            .map(|target| DefinitionLink { origin, target }.to_lsp())
            .collect();

        Ok(Some(lsp::DefinitionResponse::DefinitionLinkList(links)))
    }

    pub(in crate::server) fn type_definition(
        &mut self,
        params: lsp::TypeDefinitionParams,
    ) -> Result<Option<lsp::TypeDefinitionResponse>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };
        let locations: Vec<lsp::Location> = document
            .analyzer
            .type_definition(&document.uri, position)
            .iter()
            .map(ToLsp::to_lsp)
            .collect();

        Ok(
            (!locations.is_empty()).then_some(lsp::TypeDefinitionResponse::Definition(
                lsp::Definition::LocationList(locations),
            )),
        )
    }

    pub(in crate::server) fn references(
        &mut self,
        params: lsp::ReferenceParams,
    ) -> Result<Option<Vec<lsp::Location>>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };

        Ok(Some(
            document
                .analyzer
                .references(&document.uri, position)
                .iter()
                .map(ToLsp::to_lsp)
                .collect(),
        ))
    }

    pub(in crate::server) fn document_highlight(
        &mut self,
        params: lsp::DocumentHighlightParams,
    ) -> Result<Option<Vec<lsp::DocumentHighlight>>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };

        Ok(Some(
            document
                .analyzer
                .references(&document.uri, position)
                .iter()
                .filter(|location| location.uri == document.uri)
                .map(|location| DocumentHighlight(location.range).to_lsp())
                .collect(),
        ))
    }

    pub(in crate::server) fn prepare_rename(
        &mut self,
        params: lsp::PrepareRenameParams,
    ) -> Result<Option<lsp::PrepareRenameResult>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };

        Ok(document
            .analyzer
            .prepare_rename(&document.uri, position)
            .map(|range| lsp::PrepareRenameResult::Range(range.to_lsp())))
    }

    pub(in crate::server) fn rename(
        &mut self,
        params: lsp::RenameParams,
    ) -> Result<Option<lsp::WorkspaceEdit>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };

        if document
            .analyzer
            .prepare_rename(&document.uri, position)
            .is_none()
        {
            return Ok(None);
        }

        match document
            .analyzer
            .rename(&document.uri, position, &params.new_name)
        {
            Some(edit) => Ok(Some(edit.to_lsp())),
            None => Err(ResponseError::request_failure(format!(
                "\"{}\" is not a valid name.",
                params.new_name
            ))),
        }
    }

    pub(in crate::server) fn prepare_call_hierarchy(
        &mut self,
        params: lsp::CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<lsp::CallHierarchyItem>>> {
        let Some((document, position)) =
            AnalyzedDocument::at(self, &params.text_document_position_params)?
        else {
            return Ok(None);
        };
        let items: Vec<lsp::CallHierarchyItem> = document
            .analyzer
            .prepare_call_hierarchy(&document.uri, position)
            .iter()
            .map(ToLsp::to_lsp)
            .collect();

        Ok((!items.is_empty()).then_some(items))
    }

    pub(in crate::server) fn incoming_calls(
        &mut self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        let item = params.item;
        let Some(document) = AnalyzedDocument::of(self, &item.uri)? else {
            return Ok(None);
        };
        let position = Position::from_lsp(item.selection_range.start);

        Ok(Some(
            document
                .analyzer
                .incoming_calls(&document.uri, position)
                .iter()
                .map(|call| IncomingCall(call).to_lsp())
                .collect(),
        ))
    }

    pub(in crate::server) fn outgoing_calls(
        &mut self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        let item = params.item;
        let Some(document) = AnalyzedDocument::of(self, &item.uri)? else {
            return Ok(None);
        };
        let position = Position::from_lsp(item.selection_range.start);

        Ok(Some(
            document
                .analyzer
                .outgoing_calls(&document.uri, position)
                .iter()
                .map(|call| OutgoingCall(call).to_lsp())
                .collect(),
        ))
    }
}
