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
        let position = from_lsp::position(at.position);
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
            let locations = definitions.iter().map(to_lsp::location).collect();

            return Ok(Some(lsp::DefinitionResponse::Definition(
                lsp::Definition::LocationList(locations),
            )));
        };
        let links = definitions
            .iter()
            .map(|definition| {
                let location = to_lsp::location(definition);

                lsp::LocationLink {
                    origin_selection_range: Some(to_lsp::range(origin)),
                    target_uri: location.uri,
                    target_range: location.range,
                    target_selection_range: location.range,
                }
            })
            .collect();

        Ok(Some(lsp::DefinitionResponse::DefinitionLinkList(links)))
    }

    pub(in crate::server) fn type_definition(
        &mut self,
        params: lsp::TypeDefinitionParams,
    ) -> Result<Option<lsp::TypeDefinitionResponse>> {
        let at = params.text_document_position_params;
        let Some(document) = self.document_analysis(&at.text_document.uri)? else {
            return Ok(None);
        };
        let locations: Vec<lsp::Location> = document
            .analyzer
            .type_definition(&document.uri, from_lsp::position(at.position))
            .iter()
            .map(to_lsp::location)
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

    pub(in crate::server) fn prepare_call_hierarchy(
        &mut self,
        params: lsp::CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<lsp::CallHierarchyItem>>> {
        let at = params.text_document_position_params;
        let Some(document) = self.document_analysis(&at.text_document.uri)? else {
            return Ok(None);
        };
        let items: Vec<lsp::CallHierarchyItem> = document
            .analyzer
            .prepare_call_hierarchy(&document.uri, from_lsp::position(at.position))
            .iter()
            .map(to_lsp::call_hierarchy_item)
            .collect();

        Ok((!items.is_empty()).then_some(items))
    }

    pub(in crate::server) fn incoming_calls(
        &mut self,
        params: lsp::CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<lsp::CallHierarchyIncomingCall>>> {
        let item = params.item;
        let Some(document) = self.document_analysis(&item.uri)? else {
            return Ok(None);
        };
        let position = from_lsp::position(item.selection_range.start);

        Ok(Some(
            document
                .analyzer
                .incoming_calls(&document.uri, position)
                .iter()
                .map(|call| lsp::CallHierarchyIncomingCall {
                    from: to_lsp::call_hierarchy_item(&call.item),
                    from_ranges: call.ranges.iter().copied().map(to_lsp::range).collect(),
                })
                .collect(),
        ))
    }

    pub(in crate::server) fn outgoing_calls(
        &mut self,
        params: lsp::CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<lsp::CallHierarchyOutgoingCall>>> {
        let item = params.item;
        let Some(document) = self.document_analysis(&item.uri)? else {
            return Ok(None);
        };
        let position = from_lsp::position(item.selection_range.start);

        Ok(Some(
            document
                .analyzer
                .outgoing_calls(&document.uri, position)
                .iter()
                .map(|call| lsp::CallHierarchyOutgoingCall {
                    to: to_lsp::call_hierarchy_item(&call.item),
                    from_ranges: call.ranges.iter().copied().map(to_lsp::range).collect(),
                })
                .collect(),
        ))
    }
}
