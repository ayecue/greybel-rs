use gen_lsp_types as lsp;

use super::Server;
use crate::error::{ResponseError, Result};
use crate::{from_lsp, to_lsp};

impl Server {
    pub(in crate::server) fn definition(
        &mut self,
        params: lsp::DefinitionParams,
    ) -> Result<Option<lsp::DefinitionResponse>> {
        let at = params.text_document_position_params;
        let Some(document) = self.document_analysis(&at.text_document.uri)? else {
            return Ok(None);
        };
        let locations: Vec<lsp::Location> = document
            .analyzer
            .definition(&document.uri, from_lsp::position(at.position))
            .iter()
            .map(to_lsp::location)
            .collect();

        Ok(
            (!locations.is_empty()).then_some(lsp::DefinitionResponse::Definition(
                lsp::Definition::LocationList(locations),
            )),
        )
    }

    pub(in crate::server) fn references(
        &mut self,
        params: lsp::ReferenceParams,
    ) -> Result<Option<Vec<lsp::Location>>> {
        let at = params.text_document_position_params;
        let Some(document) = self.document_analysis(&at.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(
            document
                .analyzer
                .references(&document.uri, from_lsp::position(at.position))
                .iter()
                .map(to_lsp::location)
                .collect(),
        ))
    }

    pub(in crate::server) fn document_highlight(
        &mut self,
        params: lsp::DocumentHighlightParams,
    ) -> Result<Option<Vec<lsp::DocumentHighlight>>> {
        let at = params.text_document_position_params;
        let Some(document) = self.document_analysis(&at.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(
            document
                .analyzer
                .references(&document.uri, from_lsp::position(at.position))
                .iter()
                .filter(|location| location.uri == document.uri)
                .map(|location| lsp::DocumentHighlight {
                    range: to_lsp::range(location.range),
                    kind: Some(lsp::DocumentHighlightKind::Text),
                })
                .collect(),
        ))
    }

    pub(in crate::server) fn prepare_rename(
        &mut self,
        params: lsp::PrepareRenameParams,
    ) -> Result<Option<lsp::PrepareRenameResult>> {
        let at = params.text_document_position_params;
        let Some(document) = self.document_analysis(&at.text_document.uri)? else {
            return Ok(None);
        };

        Ok(document
            .analyzer
            .prepare_rename(&document.uri, from_lsp::position(at.position))
            .map(|range| lsp::PrepareRenameResult::Range(to_lsp::range(range))))
    }

    pub(in crate::server) fn rename(
        &mut self,
        params: lsp::RenameParams,
    ) -> Result<Option<lsp::WorkspaceEdit>> {
        let at = params.text_document_position_params;
        let Some(document) = self.document_analysis(&at.text_document.uri)? else {
            return Ok(None);
        };
        let position = from_lsp::position(at.position);

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
            Some(edit) => Ok(Some(to_lsp::workspace_edit(&edit))),
            None => Err(ResponseError::request_failure(format!(
                "\"{}\" is not a valid name",
                params.new_name
            ))),
        }
    }
}
