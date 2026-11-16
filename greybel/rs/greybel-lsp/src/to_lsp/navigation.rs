use gen_lsp_types as lsp;
use greybel_core::Range;
use greybel_type_analyzer::Location;

use super::ToLsp;

pub(crate) struct DefinitionLink<'a> {
    pub origin: Range,
    pub target: &'a Location,
}

impl ToLsp for DefinitionLink<'_> {
    type Output = lsp::LocationLink;

    fn to_lsp(&self) -> lsp::LocationLink {
        let location = self.target.to_lsp();

        lsp::LocationLink {
            origin_selection_range: Some(self.origin.to_lsp()),
            target_uri: location.uri,
            target_range: location.range,
            target_selection_range: location.range,
        }
    }
}

pub(crate) struct DocumentHighlight(pub Range);

impl ToLsp for DocumentHighlight {
    type Output = lsp::DocumentHighlight;

    fn to_lsp(&self) -> lsp::DocumentHighlight {
        lsp::DocumentHighlight {
            range: self.0.to_lsp(),
            kind: Some(lsp::DocumentHighlightKind::Text),
        }
    }
}
