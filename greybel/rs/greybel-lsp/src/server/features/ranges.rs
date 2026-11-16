use gen_lsp_types as lsp;
use greybel_core::{Position, Range};

use super::{AnalyzedDocument, Server};
use crate::error::Result;
use crate::from_lsp::FromLsp;
use crate::to_lsp::{ColorPresentation, SelectionRange, ToLsp};

impl Server {
    pub(in crate::server) fn folds(
        &mut self,
        params: lsp::FoldingRangeParams,
    ) -> Result<Option<Vec<lsp::FoldingRange>>> {
        let Some(document) = AnalyzedDocument::of(self, &params.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(
            document
                .analyzer
                .folds(&document.uri)
                .iter()
                .map(ToLsp::to_lsp)
                .collect(),
        ))
    }

    pub(in crate::server) fn selection_range(
        &mut self,
        params: lsp::SelectionRangeParams,
    ) -> Result<Option<Vec<lsp::SelectionRange>>> {
        let Some(document) = AnalyzedDocument::of(self, &params.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(
            params
                .positions
                .iter()
                .map(|&position| {
                    let ranges = document
                        .analyzer
                        .selection_ranges(&document.uri, Position::from_lsp(position));

                    SelectionRange {
                        ranges: &ranges,
                        fallback: position,
                    }
                    .to_lsp()
                })
                .collect(),
        ))
    }

    pub(in crate::server) fn inlay_hints(
        &mut self,
        params: lsp::InlayHintParams,
    ) -> Result<Option<Vec<lsp::InlayHint>>> {
        let Some(document) = AnalyzedDocument::of(self, &params.text_document.uri)? else {
            return Ok(None);
        };
        let range = Range::from_lsp(params.range);
        let options = self.settings.inlay_hint_options();

        Ok(Some(
            document
                .analyzer
                .inlay_hints(&document.uri, range, options)
                .iter()
                .map(ToLsp::to_lsp)
                .collect(),
        ))
    }

    pub(in crate::server) fn document_colors(
        &mut self,
        params: lsp::DocumentColorParams,
    ) -> Result<Vec<lsp::ColorInformation>> {
        let Some(document) = AnalyzedDocument::of(self, &params.text_document.uri)? else {
            return Ok(Vec::new());
        };

        Ok(document
            .analyzer
            .document_colors(&document.uri)
            .iter()
            .map(ToLsp::to_lsp)
            .collect())
    }

    pub(in crate::server) fn color_presentations(
        &mut self,
        params: lsp::ColorPresentationParams,
    ) -> Result<Vec<lsp::ColorPresentation>> {
        let presentation = ColorPresentation {
            color: params.color,
            range: params.range,
        };

        Ok(vec![presentation.to_lsp()])
    }
}
