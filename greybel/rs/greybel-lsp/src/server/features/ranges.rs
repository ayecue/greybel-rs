use gen_lsp_types as lsp;

use super::Server;
use crate::error::Result;
use crate::{from_lsp, to_lsp};

impl Server {
    pub(in crate::server) fn folds(
        &mut self,
        params: &lsp::FoldingRangeParams,
    ) -> Result<Option<Vec<lsp::FoldingRange>>> {
        let Some(document) = self.document_analysis(&params.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(
            document
                .analyzer
                .folds(&document.uri)
                .iter()
                .map(to_lsp::fold)
                .collect(),
        ))
    }

    pub(in crate::server) fn selection_range(
        &mut self,
        params: &lsp::SelectionRangeParams,
    ) -> Result<Option<Vec<lsp::SelectionRange>>> {
        let Some(document) = self.document_analysis(&params.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(
            params
                .positions
                .iter()
                .map(|&position| {
                    let ranges = document
                        .analyzer
                        .selection_ranges(&document.uri, from_lsp::position(position));

                    to_lsp::selection_range(&ranges, position)
                })
                .collect(),
        ))
    }

    pub(in crate::server) fn inlay_hints(
        &mut self,
        params: &lsp::InlayHintParams,
    ) -> Result<Option<Vec<lsp::InlayHint>>> {
        let Some(document) = self.document_analysis(&params.text_document.uri)? else {
            return Ok(None);
        };
        let range = from_lsp::range(params.range);
        let options = self.settings.inlay_hint_options();

        Ok(Some(
            document
                .analyzer
                .inlay_hints(&document.uri, range, options)
                .iter()
                .map(to_lsp::inlay_hint)
                .collect(),
        ))
    }

    pub(in crate::server) fn document_colors(
        &mut self,
        params: &lsp::DocumentColorParams,
    ) -> Result<Vec<lsp::ColorInformation>> {
        let Some(document) = self.document_analysis(&params.text_document.uri)? else {
            return Ok(Vec::new());
        };

        Ok(document
            .analyzer
            .document_colors(&document.uri)
            .iter()
            .map(to_lsp::color_information)
            .collect())
    }

    pub(in crate::server) fn color_presentations(
        params: &lsp::ColorPresentationParams,
    ) -> Vec<lsp::ColorPresentation> {
        vec![to_lsp::color_presentation(params.color, params.range)]
    }
}
